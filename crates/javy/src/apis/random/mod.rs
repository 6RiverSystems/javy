use crate::quickjs::{context::Intrinsic, prelude::Func, qjs, Ctx, Object};
use anyhow::{Error, Result};
use rquickjs::Function;
use std::ptr;

pub struct Random;

#[link(wasm_import_module = "wasiraptor")]
extern {
    fn log(a : i64, b : i64);
}

impl Intrinsic for Random {
    unsafe fn add_intrinsic(ctx: std::ptr::NonNull<qjs::JSContext>) {
        register(Ctx::from_raw(ctx)).expect("`Random` APIs to succeed")
    }
}

fn register(cx: Ctx) -> Result<()> {
    let globals = cx.globals();
    let math: Object<'_> = globals.get("Math").expect("Math global to be defined");
    math.set("random", Func::from(fastrand::f64))?;

    globals.set(
        "javy_logger",
        Function::new(cx.clone(), |level: String, message: String| {
            let level_ref = unsafe { write_string(1, &level) };
            let message_ref = unsafe { write_string(1 + level_ref.1, &message) };

            unsafe { log(level_ref.0, message_ref.0) }
        })
    )?;

    Ok::<_, Error>(())
}

unsafe fn write_string(offset: usize, data: &str) -> (i64, usize) {
    let dest = offset as *mut u8;
    let src = data.as_ptr();
    let len = data.len();
    
    ptr::copy_nonoverlapping(src, dest, len);

    ((new_reference(5, offset.try_into().unwrap(), len.try_into().unwrap())).try_into().unwrap(), len)
}

fn new_reference(typ: u8, offset: u32, size: u32) -> u64 {
    // Check if the size can be represented in 28 bits
    if size >= (1 << 28) {
        panic!("size {} exceeds 28 bits precision {}", size, 1 << 28);
    }

    // Shift the dataType into the highest 4 bits
    // Shift the offset into the next 32 bits
    // Use the size as is, but ensure only the lowest 28 bits are used (using bitwise AND)
    ((typ as u64) << 60) | ((offset as u64) << 28) | (size as u64 & 0xFFFFFFF)
}

#[cfg(test)]
mod tests {
    use crate::{
        quickjs::{context::EvalOptions, Value},
        Runtime,
    };
    use anyhow::{Error, Result};

    #[test]
    fn test_random() -> Result<()> {
        let runtime = Runtime::default();
        runtime.context().with(|this| {
            let mut eval_opts = EvalOptions::default();
            eval_opts.strict = false;
            this.eval_with_options("result = Math.random()", eval_opts)?;
            let result: f64 = this
                .globals()
                .get::<&str, Value<'_>>("result")?
                .as_float()
                .unwrap();
            assert!(result >= 0.0);
            assert!(result < 1.0);
            Ok::<_, Error>(())
        })?;

        Ok(())
    }
}
