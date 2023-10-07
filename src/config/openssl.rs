use error_mapper::{SystemErrorCodes, TheError, TheResult};
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};

pub fn create_openssl_builder() -> TheResult<SslAcceptorBuilder> {

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .map_err(
            |e|
                TheError::default()
                    .with_type(SystemErrorCodes::GenericError)
                    .with_content(e.to_string())
        )?;
    builder
        .set_private_key_file("certs/key.pem", SslFiletype::PEM)
        .map_err(
            |e|
                TheError::default()
                    .with_type(SystemErrorCodes::GenericError)
                    .with_content(e.to_string())
        )?;
    builder
        .set_certificate_chain_file("certs/cert.pem")
        .map_err(
            |e|
                TheError::default()
                    .with_type(SystemErrorCodes::GenericError)
                    .with_content(e.to_string())
        )?;

    Ok(builder)
}
