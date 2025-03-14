use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use portable_atomic::AtomicF32;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock};

use super::printer::PrinterEvent;

#[derive(Debug, Clone, Copy)]
pub struct Move {
    /// start velocity, zero by default
    pub start_velocity: f32,
    /// could be NaN
    pub target_velocity: f32,
    /// distance move in x
    pub x: f32,
    /// distance move in y
    pub y: f32,
    /// distance move in z
    pub z: f32,
    /// distance move in e
    pub e: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct KinematicMove {
    /// this will be the extrusion velocity if distance is 0
    pub start_velocity: f32,
    pub acceleration: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub e: f32,
}

impl KinematicMove {
    /// returns absolute distance of move
    pub fn abs_distance(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// returns duration in ms
    pub fn duration(&self) -> f32 {
        // s = ut + at^2
        // att + ut - s = 0

        let s = self.abs_distance();

        if s == 0.0 {
            return self.e / self.start_velocity;
        }

        // fast return
        if self.acceleration == 0.0 {
            return s / self.start_velocity;
        }

        // t = (-u + sqrt(u*u + 2as)) / 2a
        let u = self.start_velocity;
        let a = self.acceleration;
        (-u + (u * u + 2.0 * a * s).sqrt()) / (2.0 * a)
    }
}

/// extrusion only move
#[derive(Debug)]
pub struct ExtrusionMove {
    /// flow rate in mm/s
    pub flow: f32,
    /// distance to extrude
    pub distance: f32,
}

pub enum Action {
    Move(Move),
    /// sets velocity in mm/s
    SetVelocity(f32),
    SetBedTemp(f32),
    SetBedTempWait(f32),
    SetExtruderTemp {
        index: usize,
        temp: f32,
    },
    SetExtruderTempWait {
        index: usize,
        temp: f32,
    },
}

#[derive(Debug)]
pub enum PrinterAction {
    KinematicMove(KinematicMove),
    ExtrusionMove(ExtrusionMove),
    SetBedTemp(f32),
    SetBedTempWait(f32),
    SetExtruderTemp { index: usize, temp: f32 },
    SetExtruderTempWait { index: usize, temp: f32 },
}

pub struct ActionState {
    /// max velocity in mm/s
    pub max_velocity: AtomicF32,
    /// max accel
    pub max_accel: AtomicF32,
    /// square corner velocity in mm/s
    pub square_corner_velocity: AtomicF32,
    /// minimum cruise ratio
    pub minimum_cruise_ratio: AtomicF32,
    /// use absolute positioning
    pub absolute_position: AtomicBool,
    pub absolute_extrution: AtomicBool,
    /// current running gcode line number
    pub gcode_line: AtomicUsize,
    pub gcode_running: AtomicBool,
    pub exclude_objects: RwLock<Vec<String>>,
    /// x origin
    pub x_origin: AtomicF32,
    /// y origin
    pub y_origin: AtomicF32,
    /// z origin
    pub z_origin: AtomicF32,
    /// x position
    pub x_position: AtomicF32,
    /// y position
    pub y_position: AtomicF32,
    /// z position
    pub z_position: AtomicF32,
    /// e position
    pub e_position: AtomicF32,
}

impl ActionState {
    pub const fn new() -> Self {
        Self {
            max_velocity: AtomicF32::new(100.0),
            max_accel: AtomicF32::new(3000.0),
            square_corner_velocity: AtomicF32::new(5.0),
            minimum_cruise_ratio: AtomicF32::new(0.5),
            absolute_position: AtomicBool::new(false),
            absolute_extrution: AtomicBool::new(false),
            gcode_line: AtomicUsize::new(0),
            gcode_running: AtomicBool::new(false),
            exclude_objects: RwLock::const_new(Vec::new()),
            x_origin: AtomicF32::new(0.0),
            y_origin: AtomicF32::new(0.0),
            z_origin: AtomicF32::new(0.0),
            x_position: AtomicF32::new(f32::NAN),
            y_position: AtomicF32::new(f32::NAN),
            z_position: AtomicF32::new(f32::NAN),
            e_position: AtomicF32::new(0.0),
        }
    }
}

#[derive(Default)]
struct ActionQueueInner {
    /// first move in queue, relative position
    first_move: Option<Move>,
    first_move_accel: f32,
    next_actions: VecDeque<PrinterAction>,
}

/// The action queue functions as a trapezoid generator.
/// Moves are queued here and encoded into trapezoidle movements when
/// enough information is given
///
/// all the moves stored in queue is converted to relative position
pub struct ActionQueue {
    pub state: Arc<ActionState>,

    suspended: AtomicBool,
    event_sender: UnboundedSender<PrinterEvent>,
    inner: Mutex<ActionQueueInner>,
}

impl ActionQueue {
    pub fn new(state: Arc<ActionState>, event_sender: UnboundedSender<PrinterEvent>) -> Self {
        Self {
            state,
            suspended: AtomicBool::new(false),
            event_sender,
            inner: Default::default(),
        }
    }

    /// suspend the action queue,
    /// any push when suspended is ignored
    pub fn suspend(&self) {
        self.suspended.store(true, Ordering::SeqCst);
    }

    /// resume the action queue, start listening to pushes
    pub fn resume(&self) {
        self.suspended.store(false, Ordering::SeqCst);
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::SeqCst)
    }

    pub async fn push(&self, action: Action) {
        // does not accept push when suspended
        if self.is_suspended() {
            return;
        }

        match action {
            Action::Move(mut next_move) => {
                let mut inner = self.inner.lock().await;

                // set the max velocity
                let max_velocity = self.state.max_velocity.load(Ordering::SeqCst);

                // set the target velocity if not provided
                if next_move.target_velocity.is_nan() {
                    next_move.target_velocity = max_velocity;
                }

                // clamp the velocity
                next_move.target_velocity = next_move.target_velocity.clamp(0.1, max_velocity);

                // convert move to relative position
                if self.state.absolute_position.load(Ordering::SeqCst) {
                    if !next_move.x.is_nan() {
                        next_move.x -= self.state.x_position.load(Ordering::SeqCst);
                    }

                    if !next_move.y.is_nan() {
                        next_move.y -= self.state.y_position.load(Ordering::SeqCst);
                    }

                    if !next_move.z.is_nan() {
                        next_move.z -= self.state.z_position.load(Ordering::SeqCst);
                    }
                }

                // convert extrusion to relative
                if self.state.absolute_extrution.load(Ordering::SeqCst) {
                    if !next_move.e.is_nan() {
                        next_move.e -= self.state.e_position.load(Ordering::SeqCst);
                    }
                }

                if next_move.x.is_nan() {
                    next_move.x = 0.0;
                }
                if next_move.y.is_nan() {
                    next_move.y = 0.0;
                }
                if next_move.z.is_nan() {
                    next_move.z = 0.0;
                }
                if next_move.e.is_nan() {
                    next_move.e = 0.0;
                }

                // add the distances to state
                self.state
                    .x_position
                    .fetch_add(next_move.x, Ordering::SeqCst);
                self.state
                    .y_position
                    .fetch_add(next_move.y, Ordering::SeqCst);
                self.state
                    .z_position
                    .fetch_add(next_move.z, Ordering::SeqCst);
                self.state
                    .e_position
                    .fetch_add(next_move.e, Ordering::SeqCst);

                // encode the first move in queue if any
                if let Some(first_move) = inner.first_move.take() {
                    // encode and send the first move
                    self.encode_and_send(first_move, Some(&next_move)).await;
                    // send the remaining actions
                    while let Some(action) = inner.next_actions.pop_front() {
                        self.send_action(action).await;
                    }

                    return;
                }

                // queue is cleared.
                // next move is the new first move
                inner.first_move = Some(next_move);
                inner.first_move_accel = self.state.max_accel.load(Ordering::SeqCst);
            }
            Action::SetVelocity(f) => {
                self.state.max_velocity.store(f, Ordering::SeqCst);
            }
            Action::SetBedTemp(t) => {
                let mut inner = self.inner.lock().await;

                if inner.first_move.is_some() {
                    inner.next_actions.push_back(PrinterAction::SetBedTemp(t));
                } else {
                    // send action immediatly if queue is empty
                    self.send_action(PrinterAction::SetBedTemp(t)).await;
                }
            }
            Action::SetBedTempWait(t) => {
                self.flush().await;
                self.send_action(PrinterAction::SetBedTempWait(t)).await;
            }
            Action::SetExtruderTemp { index, temp } => {
                // acquire lock
                let mut inner = self.inner.lock().await;
                // push to queue if queue is not empty
                if inner.first_move.is_some() {
                    inner
                        .next_actions
                        .push_back(PrinterAction::SetExtruderTemp { index, temp });
                } else {
                    // send immediately if queue is empty
                    self.send_action(PrinterAction::SetExtruderTemp { index, temp })
                        .await;
                }
            }
            Action::SetExtruderTempWait { index, temp } => {
                self.flush().await;
                self.send_action(PrinterAction::SetExtruderTempWait { index, temp })
                    .await;
            }
        }
    }

    /// encodes the remaining moves in queue.
    /// should be called when a section of gcode is finished
    pub async fn flush(&self) {
        // ignore if suspended
        if self.is_suspended() {
            return;
        }

        let mut inner = self.inner.lock().await;

        if let Some(current) = inner.first_move.take() {
            self.encode_and_send(current, None).await;
        }

        while let Some(action) = inner.next_actions.pop_front() {
            self.send_action(action).await;
        }
    }

    /// encodes the move with provided next move
    async fn encode_and_send(&self, move_: Move, next_move: Option<&Move>) {}

    async fn send_action(&self, action: PrinterAction) {
        let _ = self.event_sender.send(PrinterEvent::Action(action));
    }

    /// clear the action queue
    pub async fn clear(&self) {
        let mut inner = self.inner.lock().await;
        inner.first_move = None;
        inner.next_actions.clear();
    }
}
