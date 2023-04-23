use std::{env, fmt::Display};

use futures::future;
use oci_distribution::{
    client::{ClientProtocol, PushResponse},
    errors::OciDistributionError,
    secrets::RegistryAuth,
    Reference,
};
use tracing::{debug, info, instrument};

use crate::fake::MEGABYTE;

pub enum LoadTestError {
    OciDistributionError(OciDistributionError),
    JoinError(tokio::task::JoinError),
}

impl Display for LoadTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadTestError::OciDistributionError(e) => write!(f, "OciDistributionError: {e}"),
            LoadTestError::JoinError(e) => write!(f, "JoinError: {e}"),
        }
    }
}

/// Load tests a registry by pushing images to it.
#[instrument(skip(auth, protocol))]
pub async fn load_test(
    image_count: usize,
    host: String,
    auth: RegistryAuth,
    protocol: ClientProtocol,
) -> Vec<Result<PushResponse, LoadTestError>> {
    let mut handles = Vec::new();

    for i in 0..image_count {
        info!("Kicking off push for image {i}");
        let h = tokio::task::spawn(push_reg_image(
            i,
            host.clone(),
            auth.clone(),
            protocol.clone(),
        ));
        handles.push(h);
    }
    info!("Waiting for all pushes to complete");
    let results = future::join_all(handles).await;
    let results: Vec<Result<PushResponse, LoadTestError>> = results
        .into_iter()
        .map(|r| {
            r.map_err(LoadTestError::JoinError)
                .and_then(|r| r.map_err(LoadTestError::OciDistributionError))
        })
        .collect();
    results
}

#[instrument(level = "debug", skip(auth, protocol))]
async fn push_reg_image(
    i: usize,
    reg: String,
    auth: RegistryAuth,
    protocol: ClientProtocol,
) -> Result<PushResponse, OciDistributionError> {
    let layers = crate::fake::gen_rand_layers(10 * MEGABYTE, 1);
    let image = crate::fake::gen_image(layers).unwrap();

    let reference: Reference = format!("{reg}/test/this-{i}:latest").parse().unwrap();

    let res = crate::client::push_image(
        image.layers,
        image.config,
        reference,
        image.manifest,
        &auth,
        protocol,
    )
    .await?;
    Ok(res)
}

#[instrument]
async fn pull_docker_reg_push_docker_reg() {
    let user = env::var("DOCKER_USER").unwrap();
    let password = env::var("DOCKER_PASSWORD").unwrap();
    let auth =
        oci_distribution::secrets::RegistryAuth::Basic(user.to_string(), password.to_string());

    let image_ref = "alpine:latest".parse().unwrap();

    info!("pulling image {image_ref}");
    let image = crate::client::pull_image(ClientProtocol::Https, image_ref, &auth)
        .await
        .unwrap();

    debug!("got image {image:?}");

    let reference: Reference = "lswith/alpine:latest".parse().unwrap();

    let mut manifest = image.manifest.unwrap();
    manifest.media_type = Some(oci_distribution::manifest::OCI_IMAGE_MEDIA_TYPE.to_string());

    info!("pushing image");

    let resp: PushResponse = crate::client::push_image(
        image.layers,
        image.config,
        reference,
        Some(manifest),
        &auth,
        ClientProtocol::Https,
    )
    .await
    .unwrap();

    debug!("{}", resp.manifest_url);
}
#[instrument]
async fn pull_local_push_docker_reg() {
    let image_ref = "localhost:6000/test/this:old".parse().unwrap();

    info!("pulling image {image_ref}");
    let image = crate::client::pull_image(
        ClientProtocol::Http,
        image_ref,
        &oci_distribution::secrets::RegistryAuth::Anonymous,
    )
    .await
    .unwrap();

    debug!("got image {image:?}");

    let reference: Reference = "lswith/test:latest".parse().unwrap();

    let user = env::var("DOCKER_USER").unwrap();
    let password = env::var("DOCKER_PASSWORD").unwrap();
    let auth =
        oci_distribution::secrets::RegistryAuth::Basic(user.to_string(), password.to_string());

    info!("pushing image");
    let mut manifest = image.manifest.unwrap();
    manifest.media_type = Some(oci_distribution::manifest::OCI_IMAGE_MEDIA_TYPE.to_string());

    let resp: PushResponse = crate::client::push_image(
        image.layers,
        image.config,
        reference,
        Some(manifest),
        &auth,
        ClientProtocol::Https,
    )
    .await
    .unwrap();

    debug!("{}", resp.manifest_url);
}

#[instrument]
async fn pull_docker_reg_push_local() {
    let user = env::var("DOCKER_USER").unwrap();
    let password = env::var("DOCKER_PASSWORD").unwrap();
    let auth =
        oci_distribution::secrets::RegistryAuth::Basic(user.to_string(), password.to_string());

    let image_ref = "alpine:latest".parse().unwrap();

    info!("pulling image {image_ref}");
    let image = crate::client::pull_image(ClientProtocol::Https, image_ref, &auth)
        .await
        .unwrap();

    debug!("got image {image:?}");

    let reference: Reference = "localhost:6000/test/this:old".parse().unwrap();

    let mut manifest = image.manifest.unwrap();
    manifest.media_type = Some(oci_distribution::manifest::OCI_IMAGE_MEDIA_TYPE.to_string());

    info!("pushing image");

    let resp: PushResponse = crate::client::push_image(
        image.layers,
        image.config,
        reference,
        Some(manifest),
        &oci_distribution::secrets::RegistryAuth::Anonymous,
        ClientProtocol::Http,
    )
    .await
    .unwrap();

    debug!("{}", resp.manifest_url);
}

#[instrument]
async fn pull_local_push_local() {
    let image_ref = "localhost:6000/test/this:old".parse().unwrap();

    info!("pulling image {image_ref}");
    let image = crate::client::pull_image(
        ClientProtocol::Http,
        image_ref,
        &oci_distribution::secrets::RegistryAuth::Anonymous,
    )
    .await
    .unwrap();

    debug!("got image {image:?}");

    let image_ref = "localhost:6000/test/this:new".parse().unwrap();

    info!("pushing image");
    let mut manifest = image.manifest.unwrap();
    manifest.media_type = Some(oci_distribution::manifest::OCI_IMAGE_MEDIA_TYPE.to_string());

    let resp: PushResponse = crate::client::push_image(
        image.layers,
        image.config,
        image_ref,
        Some(manifest),
        &oci_distribution::secrets::RegistryAuth::Anonymous,
        ClientProtocol::Http,
    )
    .await
    .unwrap();

    debug!("{}", resp.manifest_url);
}
