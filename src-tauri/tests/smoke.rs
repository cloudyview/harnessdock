//! End-to-end smoke test against a real `@deepseek-ai/dsh` runtime.
//!
//! Downloads the runtime from npm (~260 MB), creates an instance from the
//! blank web template, starts it, waits for the `dsh web:` ready line, checks
//! the port is listening, stops it, and captures a template.
//!
//! Run explicitly:  cargo test --test smoke -- --ignored --nocapture
//! Optional: HD_SMOKE_VERSION=0.1.1-rc.2  HD_SMOKE_KEEP=1 (keep the temp dir)

use harnessdock_lib::model::*;
use harnessdock_lib::procs::ProcManager;
use harnessdock_lib::store::Store;
use harnessdock_lib::{dsh, instances, ports, runtimes, templates};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn temp_root() -> PathBuf {
    let p = std::env::temp_dir().join(format!("hd-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
#[ignore]
fn create_start_stop_capture() {
    let version = std::env::var("HD_SMOKE_VERSION").unwrap_or_else(|_| "0.1.1-rc.2".into());
    let root = temp_root();
    println!("smoke root: {}", root.display());
    let s = |p: PathBuf| p.to_string_lossy().to_string();

    let mut store = Store::open(root.join("data"));
    store.reg.settings.inst_root = s(root.join("instances"));
    store.reg.settings.ws_root = s(root.join("workspaces"));
    store.reg.settings.rt_root = s(root.join("runtimes"));
    store.reg.settings.pool_start = 41500;
    store.reg.settings.pool_end = 41600;
    store.save().unwrap();

    // 1. runtime
    let t0 = Instant::now();
    runtimes::install(&store.reg.settings.rt_root, &version, &store.reg.settings.registry).expect("runtime install");
    println!("runtime installed in {:?}", t0.elapsed());
    assert!(dsh::runtime_installed(&store.reg.settings.rt_root, &version));
    store.set_default_runtime(Some(version.clone()));
    store.save().unwrap();

    // 2. create from blank web template
    let inst = instances::create(
        &mut store,
        CreateReq { id: "smoke".into(), display: Some("smoke".into()), template: "blank-web".into(), runtime: version.clone(), home: None, cwd: None, port: None, model: None },
    )
    .expect("create instance");
    println!("created: port={} home={}", inst.port, inst.home);
    assert!(Path::new(&inst.home).join("profiles").join("web").join("package.json").exists(), "profile materialised");
    assert!(store.dump_path("smoke").exists(), "dump saved");

    // 3. model override writes a marked block and validates
    store.reg.providers.push(Provider {
        id: "gw".into(), name: "Gateway".into(), api: "openai-completions".into(), base_url: "https://example.invalid/v1".into(),
        key_env: "GW_API_KEY".into(), models: vec!["some-model".into()], context_window: None, max_tokens: None, builtin: false,
    });
    instances::apply_model(&mut store, "smoke", Some(ModelRef { provider: "gw".into(), model: "some-model".into() })).unwrap();
    let patch = instances::patch_read(&store, "smoke").unwrap();
    assert!(patch.contains("# harnessdock:begin model"));
    assert!(patch.contains("baseURL: https://example.invalid/v1"));
    let v = instances::validate(&store, "smoke").unwrap();
    assert!(v.ok, "dump-config after model patch: {}", v.output);
    assert!(v.output.contains("gw"), "provider row present in composed tree");
    println!("validate: {} rows, baseline diff {:?}", v.rows, v.baseline_diff);
    instances::apply_model(&mut store, "smoke", None).unwrap();

    // 4. start, wait for ready, port listening
    let procs = ProcManager::default();
    let inst = store.instance("smoke").unwrap().clone();
    let sset = store.reg.settings.clone();
    let bin = dsh::bin_js(&sset.rt_root, &inst.runtime);
    procs.start("smoke", &sset.node_path, &bin, Path::new(&inst.home), Path::new(&inst.cwd), &inst.profile, inst.port, &store.log_path("smoke")).expect("spawn");
    let t1 = Instant::now();
    let st = loop {
        let st = procs.state("smoke");
        if st.status != Status::Starting || t1.elapsed() > Duration::from_secs(120) {
            break st;
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    let log = std::fs::read_to_string(store.log_path("smoke")).unwrap_or_default();
    assert_eq!(st.status, Status::Running, "instance should be running; log:\n{log}");
    println!("ready in {:?}: {:?} pid={:?}", t1.elapsed(), st.url, st.pid);
    assert!(TcpStream::connect(("127.0.0.1", inst.port)).is_ok(), "port {} accepts connections", inst.port);
    let live = ports::listening();
    assert_eq!(live.get(&inst.port).copied(), st.pid, "netstat attributes the port to our pid");

    // 5. stop kills the tree and frees the port
    procs.stop("smoke", &store.log_path("smoke")).expect("stop");
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(procs.state("smoke").status, Status::Stopped);
    assert!(!ports::listening().contains_key(&inst.port), "port released");

    // 6. capture template, create a second instance from it
    let tpl = templates::capture(&mut store, "smoke", "smoke tpl", true).expect("capture");
    assert!(tpl.has_profile);
    assert!(store.template_dir(&tpl.id).join("baseline.dump.yml").exists());
    let inst2 = instances::create(
        &mut store,
        CreateReq { id: "smoke-2".into(), display: None, template: tpl.id.clone(), runtime: version.clone(), home: None, cwd: None, port: None, model: None },
    )
    .expect("create from captured template");
    assert_ne!(inst2.port, inst.port);
    let v2 = instances::validate(&store, "smoke-2").unwrap();
    assert!(v2.ok);
    println!("second instance baseline diff: {:?}", v2.baseline_diff);
    assert_eq!(v2.baseline_diff, Some(0), "instance from template matches baseline");

    // 7. soft delete -> trash -> restore
    instances::delete(&mut store, "smoke-2", false).unwrap();
    assert_eq!(store.reg.trash.len(), 1);
    let back = instances::restore(&mut store, "smoke-2").unwrap();
    assert!(Path::new(&back.home).join("profiles").exists());

    // copy evidence next to the repo for the verification folder
    if let Ok(dst) = std::env::var("HD_SMOKE_EVIDENCE") {
        let d = PathBuf::from(dst);
        std::fs::create_dir_all(&d).ok();
        std::fs::copy(store.log_path("smoke"), d.join("smoke.out.log")).ok();
        std::fs::copy(store.dump_path("smoke"), d.join("smoke.dump.yml")).ok();
        std::fs::copy(root.join("data").join("registry.json"), d.join("registry.json")).ok();
        std::fs::write(d.join("patch-after-model.yml"), patch).ok();
    }
    if std::env::var("HD_SMOKE_KEEP").is_err() {
        let _ = std::fs::remove_dir_all(&root);
    }
}
