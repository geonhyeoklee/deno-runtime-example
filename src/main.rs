use std::env;
use std::rc::Rc;

use deno_core::op2;
use deno_core::error::AnyError;
use deno_core::ModuleLoadResponse;
use deno_core::ModuleSourceCode;
use deno_core::PollEventLoopOptions;

use deno_ast::MediaType;
use deno_ast::ParseParams;

struct TsModuleLoader;

impl deno_core::ModuleLoader for TsModuleLoader  {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
      ) -> Result<deno_core::ModuleSpecifier, deno_core::anyhow::Error> {
        deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
    }

    fn load(
        &self,
        module_specifier: &deno_core::ModuleSpecifier,
        _maybe_referrer: Option<&deno_core::ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
      ) -> ModuleLoadResponse  {
            let module_specifier = module_specifier.clone();

            let module_load = Box::pin(async move {
                let path = module_specifier.to_file_path().unwrap();

                let media_type = MediaType::from_path(&path);
                let (module_type, should_transpile) = match MediaType::from_path(&path) {
                    MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                        (deno_core::ModuleType::JavaScript, false)
                    }
                    MediaType::Jsx => {
                        (deno_core::ModuleType::JavaScript, true)
                    }
                    MediaType::Json => {
                        (deno_core::ModuleType::Json, false)
                    }
                    MediaType::TypeScript | MediaType::Cts | MediaType::Dts | MediaType::Mts | MediaType::Dmts | MediaType::Dcts | MediaType::Tsx => {
                        (deno_core::ModuleType::JavaScript, true)
                    }
                    _ => panic!("Unknown extension {:?}", path.extension())
                };
    
                let code = std::fs::read_to_string(&path)?;
                let code = if should_transpile {
                    let parsed = deno_ast::parse_module(ParseParams {
                        specifier: module_specifier.clone(),
                        text: code.into(),
                        media_type,
                        capture_tokens: false,
                        scope_analysis: false,
                        maybe_syntax: None,
                      })?;
                      String::from_utf8(parsed
                        .transpile(&Default::default(), &Default::default())?
                        .into_source().source)?
                } else {
                    code
                };
    
                let module = deno_core::ModuleSource::new(
                    module_type,
                    ModuleSourceCode::String(code.into()),
                    &module_specifier,
                    None,
                );
                Ok(module)
            });
            ModuleLoadResponse::Async(module_load)
    }
}

#[op2(async)]
#[string]
async fn op_read_file(#[string] path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op2(async)]
async fn op_write_file(#[string] path: String, #[string] contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op2(fast)]
fn op_remove_file(#[string] path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, AnyError> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}

// globalThis.runjs에 주입할 API를 생성하는 매크로이다.
// deno_core의 ops_builtin.rs에 없는 함수를 추가할 때 Extension 구조체를 만들어서 주입한다.
deno_core::extension! {
    runjs,
    ops = [
        op_read_file,
        op_write_file,
        op_remove_file,
        op_fetch,
    ]
}

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

async fn run_js(file_path: &str) -> Result<(), AnyError> {
    let main_module = deno_core::resolve_path(file_path, env::current_dir()?.as_path())?;
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(TsModuleLoader)),
        extensions: vec![runjs::init_ops()],
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        ..Default::default()
    });

    // globalThis에 API 정의
    // js_runtime.execute_script("[runjs:runtime.js]", include_str!("runtime.js")).unwrap();

    let mod_id = js_runtime.load_main_es_module(&main_module).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(PollEventLoopOptions::default()).await?;
    result.await?;

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.is_empty() {
        eprintln!("Usage: runjs <file>");
        std::process::exit(1);
    }

    let file_path = &args[1];


    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    if let Err(error) = runtime.block_on(run_js(file_path)) {
        eprintln!("error: {:?}", error);
    }

}
