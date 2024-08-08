use std::env;
use std::rc::Rc;

use deno_core::op2;
use deno_core::error::AnyError;
use deno_core::PollEventLoopOptions;

#[op2(async)]
#[string]
async fn op_read_file(#[string] path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op2(async)]
#[string]
async fn op_write_file(#[string] path: String, #[string] contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op2(fast)]
#[string]
fn op_remove_file(#[string] path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

// globalThis.runjs에 주입할 API를 생성하는 매크로이다.
// deno_core의 ops_builtin.rs에 없는 함수를 추가할 때 Extension 구조체를 만들어서 주입한다.
deno_core::extension! {
    runjs,
    ops = [
        op_read_file,
        op_write_file,
        op_remove_file,
    ]
}

async fn run_js(file_path: &str) -> Result<(), AnyError> {
    let main_module = deno_core::resolve_path(file_path, env::current_dir()?.as_path())?;
    
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![runjs::init_ops()],
        ..Default::default()
    });

    // globalThis에 API 정의
    js_runtime.execute_script("[runjs:runtime.js]", include_str!("runtime.js")).unwrap();

    let mod_id = js_runtime.load_main_es_module(&main_module).await?;
    let result = js_runtime.mod_evaluate(mod_id);

    js_runtime.run_event_loop(PollEventLoopOptions::default()).await?;
    result.await?;

    Ok(())
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    if let Err(error) = runtime.block_on(run_js("src/example.js")) {
        eprintln!("error: {:?}", error);
    }

}
