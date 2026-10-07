// cargo-generate does not render placeholder defaults, so the service_name default arrives as
// the literal text "{{project-name}}". Replace it with the real project name (--define wins).
let service = variable::get("service_name");
if service == "" || service.contains("project-name") {
    variable::set("service_name", variable::get("project-name"));
}
