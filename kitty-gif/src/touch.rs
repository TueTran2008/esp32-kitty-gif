//use cst816s::command::{IrqCtl, MotionMask, TouchEvent};
use cst816s::*;
use esp_idf_hal::gpio::{AnyIOPin, OutputPin, PinDriver};
use esp_idf_hal::task::block_on;
use esp_idf_hal::{delay::Delay, i2c};
use shared_bus::BusManager;
use std::sync::{Arc, Mutex};

