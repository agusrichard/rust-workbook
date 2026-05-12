pub mod futures_syntax;
pub mod concurrency_with_async;

use futures_syntax::run as fs_run;
use concurrency_with_async::run as cwa_run;

fn main() {
    // fs_run();
    cwa_run();
}

