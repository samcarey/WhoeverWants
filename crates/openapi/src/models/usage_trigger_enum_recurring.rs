/*
 * Twilio - Api
 *
 * This is the public Twilio REST API.
 *
 * The version of the OpenAPI document: 1.55.0
 * Contact: support@twilio.com
 * Generated by: https://openapi-generator.tech
 */


/// 
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum UsageTriggerEnumRecurring {
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "monthly")]
    Monthly,
    #[serde(rename = "yearly")]
    Yearly,
    #[serde(rename = "alltime")]
    Alltime,

}

impl ToString for UsageTriggerEnumRecurring {
    fn to_string(&self) -> String {
        match self {
            Self::Daily => String::from("daily"),
            Self::Monthly => String::from("monthly"),
            Self::Yearly => String::from("yearly"),
            Self::Alltime => String::from("alltime"),
        }
    }
}

impl Default for UsageTriggerEnumRecurring {
    fn default() -> UsageTriggerEnumRecurring {
        Self::Daily
    }
}



