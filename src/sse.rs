use regex::{Regex, RegexBuilder};

pub fn get_sse_re() -> Regex {
    RegexBuilder::new(r"^(?:event:\s(\w+)\n)?data:\s(.*)$")
        .multi_line(true)
        .build()
        .unwrap()
}

pub struct SSEvent {
    pub event: Option<String>,
    pub data: String,
}

pub fn convert_sse(re: &Regex, message: String) -> Option<SSEvent> {
    // Empty messages are ok
    if message == "" {
        return None;
    }

    let caps = re.captures(&message).unwrap();

    let event = caps.get(1).map(|m| m.as_str().to_owned());
    let data = caps.get(2).unwrap().as_str().to_owned();
    Some(SSEvent { event, data })
}
