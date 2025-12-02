use flow_like_types::{FromProto, ToProto};

use crate::flow::{
    node::{FnRefs, Node, NodeScores},
    oauth::OAuthProvider,
    pin::Pin,
};

impl ToProto<flow_like_types::proto::NodeScores> for NodeScores {
    fn to_proto(&self) -> flow_like_types::proto::NodeScores {
        flow_like_types::proto::NodeScores {
            privacy: self.privacy as u32,
            security: self.security as u32,
            performance: self.performance as u32,
            governance: self.governance as u32,
            reliability: self.reliability as u32,
            cost: self.cost as u32,
        }
    }
}

impl FromProto<flow_like_types::proto::NodeScores> for NodeScores {
    fn from_proto(proto: flow_like_types::proto::NodeScores) -> Self {
        NodeScores {
            privacy: proto.privacy as u8,
            security: proto.security as u8,
            performance: proto.performance as u8,
            governance: proto.governance as u8,
            reliability: proto.reliability as u8,
            cost: proto.cost as u8,
        }
    }
}

impl ToProto<flow_like_types::proto::FnRefs> for FnRefs {
    fn to_proto(&self) -> flow_like_types::proto::FnRefs {
        flow_like_types::proto::FnRefs {
            fn_refs: self.fn_refs.clone(),
            can_reference_fns: self.can_reference_fns,
            can_be_referenced_by_fns: self.can_be_referenced_by_fns,
        }
    }
}

impl FromProto<flow_like_types::proto::FnRefs> for FnRefs {
    fn from_proto(proto: flow_like_types::proto::FnRefs) -> Self {
        FnRefs {
            fn_refs: proto.fn_refs,
            can_reference_fns: proto.can_reference_fns,
            can_be_referenced_by_fns: proto.can_be_referenced_by_fns,
        }
    }
}

impl ToProto<flow_like_types::proto::OAuthProvider> for OAuthProvider {
    fn to_proto(&self) -> flow_like_types::proto::OAuthProvider {
        flow_like_types::proto::OAuthProvider {
            id: self.id.clone(),
            name: self.name.clone(),
            auth_url: self.auth_url.clone(),
            token_url: self.token_url.clone(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            scopes: self.scopes.clone(),
            pkce_required: self.pkce_required,
            revoke_url: self.revoke_url.clone(),
            userinfo_url: self.userinfo_url.clone(),
            oidc_discovery_url: self.oidc_discovery_url.clone(),
            jwks_url: self.jwks_url.clone(),
            audience: self.audience.clone(),
            device_auth_url: self.device_auth_url.clone(),
            use_device_flow: self.use_device_flow,
        }
    }
}

impl FromProto<flow_like_types::proto::OAuthProvider> for OAuthProvider {
    fn from_proto(proto: flow_like_types::proto::OAuthProvider) -> Self {
        OAuthProvider {
            id: proto.id,
            name: proto.name,
            auth_url: proto.auth_url,
            token_url: proto.token_url,
            client_id: proto.client_id,
            client_secret: proto.client_secret,
            scopes: proto.scopes,
            pkce_required: proto.pkce_required,
            revoke_url: proto.revoke_url,
            userinfo_url: proto.userinfo_url,
            oidc_discovery_url: proto.oidc_discovery_url,
            jwks_url: proto.jwks_url,
            audience: proto.audience,
            device_auth_url: proto.device_auth_url,
            use_device_flow: proto.use_device_flow,
        }
    }
}

impl ToProto<flow_like_types::proto::Node> for Node {
    fn to_proto(&self) -> flow_like_types::proto::Node {
        let (coord_x, coord_y, coord_z) = self.coordinates.unwrap_or((0.0, 0.0, 0.0));
        flow_like_types::proto::Node {
            id: self.id.clone(),
            name: self.name.clone(),
            friendly_name: self.friendly_name.clone(),
            description: self.description.clone(),
            coord_x,
            coord_y,
            coord_z,
            category: self.category.clone(),
            scores: self.scores.as_ref().map(|s| s.to_proto()),
            pins: self
                .pins
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            start: self.start.unwrap_or(false),
            icon: self.icon.clone().unwrap_or_default(),
            comment: self.comment.clone(),
            long_running: self.long_running.unwrap_or(false),
            error: self.error.clone(),
            docs: self.docs.clone(),
            layer: self.layer.clone(),
            event_callback: self.event_callback.unwrap_or(false),
            hash: self.hash,
            fn_refs: self.fn_refs.as_ref().map(|f| f.to_proto()),
            oauth_providers: self
                .oauth_providers
                .as_ref()
                .map(|providers| providers.iter().map(|p| p.to_proto()).collect())
                .unwrap_or_default(),
            required_oauth_scopes: self
                .required_oauth_scopes
                .as_ref()
                .map(|scopes| {
                    scopes
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                flow_like_types::proto::StringList { values: v.clone() },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl FromProto<flow_like_types::proto::Node> for Node {
    fn from_proto(proto: flow_like_types::proto::Node) -> Self {
        Node {
            id: proto.id,
            name: proto.name,
            friendly_name: proto.friendly_name,
            description: proto.description,
            coordinates: Some((proto.coord_x, proto.coord_y, proto.coord_z)),
            category: proto.category,
            scores: proto.scores.map(NodeScores::from_proto),
            pins: proto
                .pins
                .into_iter()
                .map(|(k, v)| (k, Pin::from_proto(v)))
                .collect(),
            start: if proto.start { Some(true) } else { None },
            icon: if proto.icon.is_empty() {
                None
            } else {
                Some(proto.icon)
            },
            comment: proto.comment,
            long_running: if proto.long_running { Some(true) } else { None },
            error: proto.error,
            docs: proto.docs,
            event_callback: if proto.event_callback {
                Some(true)
            } else {
                None
            },
            layer: proto.layer,
            hash: proto.hash,
            fn_refs: proto.fn_refs.map(FnRefs::from_proto),
            oauth_providers: if proto.oauth_providers.is_empty() {
                None
            } else {
                Some(
                    proto
                        .oauth_providers
                        .into_iter()
                        .map(OAuthProvider::from_proto)
                        .collect(),
                )
            },
            required_oauth_scopes: if proto.required_oauth_scopes.is_empty() {
                None
            } else {
                Some(
                    proto
                        .required_oauth_scopes
                        .into_iter()
                        .map(|(k, v)| (k, v.values))
                        .collect(),
                )
            },
        }
    }
}
