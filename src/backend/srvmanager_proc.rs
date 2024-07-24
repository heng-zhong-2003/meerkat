use inline_colorization::*;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};
use tokio::io::{self, AsyncReadExt};

use crate::backend::varworker_proc;
use crate::frontend::{meerast, parse, typecheck};
use tokio::{sync::mpsc, task::JoinHandle};

const BUFFER_SIZE: usize = 1024;
