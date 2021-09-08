use std::str::FromStr;

use cron::Schedule;

#[test]
fn issue_86() {
    let expression = "@dailyBla";
    assert!(Schedule::from_str(expression).is_err());
    let expression = " @dailyBla ";
    assert!(Schedule::from_str(expression).is_err());
}
