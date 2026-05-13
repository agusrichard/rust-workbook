pub mod futures_syntax;
pub mod concurrency_with_async;
pub mod more_futures;

use futures_syntax::run as fs_run;
use concurrency_with_async::run as cwa_run;
use more_futures::run as mf_run;

fn main() {
    // fs_run();
    // cwa_run();
    mf_run()
}

