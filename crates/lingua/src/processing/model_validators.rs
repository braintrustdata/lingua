use crate::universal::UniversalRequest;

type RequestShapeRejector = fn(&UniversalRequest) -> Option<&'static str>;

const REQUEST_VALIDATORS: &[(&str, RequestShapeRejector)] =
    &[("glm-5.2", reject_glm_5_2_request_shape)];

pub(crate) fn model_has_request_validator(model: &str) -> bool {
    request_validator(model).is_some()
}

pub(crate) fn reject_reason_for_model(
    model: &str,
    request: &UniversalRequest,
) -> Option<&'static str> {
    request_validator(model).and_then(|validator| validator(request))
}

fn request_validator(model: &str) -> Option<RequestShapeRejector> {
    REQUEST_VALIDATORS
        .iter()
        .find_map(|(registered_model, validator)| {
            (*registered_model == model).then_some(*validator)
        })
}

fn reject_glm_5_2_request_shape(_request: &UniversalRequest) -> Option<&'static str> {
    None
}
