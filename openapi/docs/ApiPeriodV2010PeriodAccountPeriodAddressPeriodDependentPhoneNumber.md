# ApiPeriodV2010PeriodAccountPeriodAddressPeriodDependentPhoneNumber

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**sid** | Option<**String**> | The unique string that that we created to identify the DependentPhoneNumber resource. | [optional]
**account_sid** | Option<**String**> | The SID of the [Account](https://www.twilio.com/docs/iam/api/account) that created the DependentPhoneNumber resource. | [optional]
**friendly_name** | Option<**String**> | The string that you assigned to describe the resource. | [optional]
**phone_number** | Option<**String**> | The phone number in [E.164](https://www.twilio.com/docs/glossary/what-e164) format, which consists of a + followed by the country code and subscriber number. | [optional]
**voice_url** | Option<**String**> | The URL we call when the phone number receives a call. The `voice_url` will not be used if a `voice_application_sid` or a `trunk_sid` is set. | [optional]
**voice_method** | Option<**String**> | The HTTP method we use to call `voice_url`. Can be: `GET` or `POST`. | [optional]
**voice_fallback_method** | Option<**String**> | The HTTP method we use to call `voice_fallback_url`. Can be: `GET` or `POST`. | [optional]
**voice_fallback_url** | Option<**String**> | The URL that we call when an error occurs retrieving or executing the TwiML requested by `url`. | [optional]
**voice_caller_id_lookup** | Option<**bool**> | Whether we look up the caller's caller-ID name from the CNAM database. Can be: `true` or `false`. Caller ID lookups can cost $0.01 each. | [optional]
**date_created** | Option<**String**> | The date and time in GMT that the resource was created specified in [RFC 2822](https://www.ietf.org/rfc/rfc2822.txt) format. | [optional]
**date_updated** | Option<**String**> | The date and time in GMT that the resource was last updated specified in [RFC 2822](https://www.ietf.org/rfc/rfc2822.txt) format. | [optional]
**sms_fallback_method** | Option<**String**> | The HTTP method we use to call `sms_fallback_url`. Can be: `GET` or `POST`. | [optional]
**sms_fallback_url** | Option<**String**> | The URL that we call when an error occurs while retrieving or executing the TwiML from `sms_url`. | [optional]
**sms_method** | Option<**String**> | The HTTP method we use to call `sms_url`. Can be: `GET` or `POST`. | [optional]
**sms_url** | Option<**String**> | The URL we call when the phone number receives an incoming SMS message. | [optional]
**address_requirements** | Option<[**crate::models::DependentPhoneNumberEnumAddressRequirement**](dependent_phone_number_enum_address_requirement.md)> |  | [optional]
**capabilities** | Option<[**serde_json::Value**](.md)> | The set of Boolean properties that indicates whether a phone number can receive calls or messages.  Capabilities are  `Voice`, `SMS`, and `MMS` and each capability can be: `true` or `false`. | [optional]
**status_callback** | Option<**String**> | The URL we call using the `status_callback_method` to send status information to your application. | [optional]
**status_callback_method** | Option<**String**> | The HTTP method we use to call `status_callback`. Can be: `GET` or `POST`. | [optional]
**api_version** | Option<**String**> | The API version used to start a new TwiML session. | [optional]
**sms_application_sid** | Option<**String**> | The SID of the application that handles SMS messages sent to the phone number. If an `sms_application_sid` is present, we ignore all `sms_*_url` values and use those of the application. | [optional]
**voice_application_sid** | Option<**String**> | The SID of the application that handles calls to the phone number. If a `voice_application_sid` is present, we ignore all of the voice urls and use those set on the application. Setting a `voice_application_sid` will automatically delete your `trunk_sid` and vice versa. | [optional]
**trunk_sid** | Option<**String**> | The SID of the Trunk that handles calls to the phone number. If a `trunk_sid` is present, we ignore all of the voice urls and voice applications and use those set on the Trunk. Setting a `trunk_sid` will automatically delete your `voice_application_sid` and vice versa. | [optional]
**emergency_status** | Option<[**crate::models::DependentPhoneNumberEnumEmergencyStatus**](dependent_phone_number_enum_emergency_status.md)> |  | [optional]
**emergency_address_sid** | Option<**String**> | The SID of the emergency address configuration that we use for emergency calling from the phone number. | [optional]
**uri** | Option<**String**> | The URI of the resource, relative to `https://api.twilio.com`. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


