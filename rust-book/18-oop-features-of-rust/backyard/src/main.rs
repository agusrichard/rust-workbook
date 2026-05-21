mod what_is_oo;
mod trait_objects;
mod tradional_oo;

use what_is_oo::run as wio_run;
use trait_objects::run as to_run;
use tradional_oo::run as oodp_run;

fn main() {
    // wio_run();
    // to_run();
    oodp_run();
}
