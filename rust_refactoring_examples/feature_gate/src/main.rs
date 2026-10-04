mod config_rules;
mod evaluator;
mod request_rules;
mod rules;

use std::collections::HashMap;

use config_rules::ConfigRules;
use evaluator::Evaluator;
use request_rules::RequestRules;

#[macro_export]
macro_rules! audit {
    ($source:expr, $flag:expr, $message:expr) => {
        println!("[audit] source={} flag={} {}", $source, $flag, $message)
    };
}

fn main() {
    let config = ConfigRules::new(HashMap::from([(
        "new-checkout".to_string(),
        "team=payments".to_string(),
    )]));
    let request = RequestRules::new(HashMap::from([(
        "new-checkout".to_string(),
        "user-42".to_string(),
    )]));

    let evaluator = Evaluator::new(vec![Box::new(request), Box::new(config)]);
    let context = HashMap::from([
        ("user".to_string(), "user-42".to_string()),
        ("team".to_string(), "payments".to_string()),
    ]);

    let result = evaluator.evaluate("new-checkout", &context);
    println!("enabled={} reason={}", result.enabled, result.reason);
}
