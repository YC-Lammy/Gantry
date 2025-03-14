use std::pin::Pin;

use crate::printer::action::{Action, Move};

use super::vm::GcodeVM;

pub fn handler<'a>(
    vm: &'a GcodeVM,
    params: &'a [String],
) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + Sync + 'a>> {
    Box::pin(handler_inner(vm, params))
}

async fn handler_inner(vm: &GcodeVM, params: &[String]) -> anyhow::Result<String> {
    let mut move_ = Move {
        start_velocity: f32::NAN,
        target_velocity: f32::NAN,
        x: f32::NAN,
        y: f32::NAN,
        z: f32::NAN,
        e: f32::NAN,
    };

    for param in params {
        if param.starts_with('X') || param.starts_with('x') {
            move_.x = fast_float::parse(&param[1..])?;
        }
        if param.starts_with('Y') || param.starts_with('y') {
            move_.y = fast_float::parse(&param[1..])?;
        }
        if param.starts_with('Z') || param.starts_with('Z') {
            move_.z = fast_float::parse(&param[1..])?;
        }
        if param.starts_with('E') || param.starts_with('e') {
            move_.e = fast_float::parse(&param[1..])?;
        }
        if param.starts_with('F') || param.starts_with('f') {
            move_.target_velocity = fast_float::parse::<f32, _>(&param[1..])? / 60.0;
        }
    }

    if move_.x.is_nan() && move_.y.is_nan() && move_.z.is_nan() && move_.e.is_nan() {
        if !move_.target_velocity.is_nan() {
            vm.action_queue
                .push(Action::SetVelocity(move_.target_velocity))
                .await;
            return Ok(String::new());
        }

        return Ok(String::new());
    }

    vm.action_queue.push(Action::Move(move_)).await;

    return Ok(String::new());
}
