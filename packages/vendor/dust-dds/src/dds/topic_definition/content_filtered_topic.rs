use crate::{
    dds_async::content_filtered_topic::ContentFilteredTopicAsync,
    domain::domain_participant::DomainParticipant,
    infrastructure::error::DdsResult,
    std_runtime::executor::block_on,
    topic_definition::{topic::Topic, topic_description::TopicDescription},
};
use alloc::{string::String, vec::Vec};

/// [`ContentFilteredTopic`] describes a more sophisticated subscription that indicates the subscriber does not want to necessarily see
/// all values of each instance published under the [`Topic`]. Rather, it wants to see only the values whose contents satisfy certain
/// criteria. This class therefore can be used to request content-based subscriptions.
#[derive(Clone)]
pub struct ContentFilteredTopic {
    topic: ContentFilteredTopicAsync,
}

impl From<ContentFilteredTopicAsync> for ContentFilteredTopic {
    fn from(value: ContentFilteredTopicAsync) -> Self {
        Self { topic: value }
    }
}

impl From<ContentFilteredTopic> for ContentFilteredTopicAsync {
    fn from(value: ContentFilteredTopic) -> Self {
        value.topic
    }
}

impl ContentFilteredTopic {
    /// This operation returns the [`Topic`] associated with the ContentFilteredTopic. That is, the
    /// [`Topic`] specified when the [`ContentFilteredTopic`] was created.
    pub fn get_related_topic(&self) -> Topic {
        self.topic.get_related_topic().clone().into()
    }

    /// This operation returns the expression_parameters associated with the [`ContentFilteredTopic`]. That is, the parameters specified
    /// on the last successful call to set_expression_parameters, or if set_expression_parameters was never called, the parameters
    /// specified when the [`ContentFilteredTopic`] was created.
    pub fn get_expression_parameters(&self) -> DdsResult<Vec<String>> {
        block_on(self.topic.get_expression_parameters())
    }

    /// This operation changes the expression_parameters associated with the  [`ContentFilteredTopic`].
    pub fn set_expression_parameters(&self, expression_parameters: Vec<String>) -> DdsResult<()> {
        block_on(self.topic.set_expression_parameters(expression_parameters))
    }
}

impl TopicDescription for ContentFilteredTopic {
    fn get_participant(&self) -> DomainParticipant {
        self.topic.get_participant()
    }

    fn get_type_name(&self) -> String {
        self.topic.get_type_name()
    }

    fn get_name(&self) -> String {
        self.topic.get_name()
    }
}
