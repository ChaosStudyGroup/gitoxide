mod connect_message {
    use git_transport::{client::git, Protocol, Service};

    #[test]
    fn version_1_without_host_and_version() {
        assert_eq!(
            git::connect_message(Service::UploadPack, Protocol::V1, b"hello/world", None),
            "git-upload-pack hello/world\0"
        )
    }
    #[test]
    fn version_2_without_host_and_version() {
        assert_eq!(
            git::connect_message(Service::UploadPack, Protocol::V2, b"hello\\world", None),
            "git-upload-pack hello\\world\0\0version=2\0"
        )
    }
    #[test]
    fn with_host_without_port() {
        assert_eq!(
            git::connect_message(
                Service::UploadPack,
                Protocol::V1,
                b"hello\\world",
                Some(&("host".into(), None))
            ),
            "git-upload-pack hello\\world\0host=host\0"
        )
    }
    #[test]
    fn with_host_with_port() {
        assert_eq!(
            git::connect_message(
                Service::UploadPack,
                Protocol::V1,
                b"hello\\world",
                Some(&("host".into(), Some(404)))
            ),
            "git-upload-pack hello\\world\0host=host:404\0"
        )
    }
}

mod upload_pack {
    use crate::fixture_bytes;
    use git_transport::{client::TransportSketch, Protocol, Service};
    use std::io::BufRead;

    #[test]
    #[ignore]
    fn clone_v1() -> crate::Result {
        let mut out = Vec::new();
        let input = fixture_bytes("v1/clone.response");
        let mut c = git_transport::client::git::Connection::new(
            input.as_slice(),
            &mut out,
            Protocol::V1,
            "/foo.git",
            Some(("example.org", None)),
        );
        let res = c.set_service(Service::UploadPack)?;
        assert_eq!(res.actual_protocol, Protocol::V1);
        // assert_eq!(res.capabilities, vec!["hello"].into());
        let refs = res
            .refs
            .expect("v1 protocol provides refs")
            .lines()
            .flat_map(Result::ok)
            .collect::<Vec<_>>();
        assert_eq!(refs, vec!["HEAD"]);
        Ok(())
    }

    #[test]
    fn upload_pack_clone_v2() {
        // With port
        // it lists the version in the first line
    }
    #[test]
    #[ignore]
    fn upload_pack_clone_version_unsupported() {
        // it replies with version 1, but doesn't list the version number, we can't test it actually, that's alright
    }
}
