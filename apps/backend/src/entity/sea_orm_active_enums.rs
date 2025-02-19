//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.4

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "BitType")]
pub enum BitType {
    #[sea_orm(string_value = "LLM")]
    Llm,
    #[sea_orm(string_value = "VLM")]
    Vlm,
    #[sea_orm(string_value = "EMBEDDING")]
    Embedding,
    #[sea_orm(string_value = "IMAGE_EMBEDDING")]
    ImageEmbedding,
    #[sea_orm(string_value = "FILE")]
    File,
    #[sea_orm(string_value = "MEDIA")]
    Media,
    #[sea_orm(string_value = "TEMPLATE")]
    Template,
    #[sea_orm(string_value = "TOKENIZER")]
    Tokenizer,
    #[sea_orm(string_value = "TOKENIZER_CONFIG")]
    TokenizerConfig,
    #[sea_orm(string_value = "SPECIAL_TOKENS_MAP")]
    SpecialTokensMap,
    #[sea_orm(string_value = "CONFIG")]
    Config,
    #[sea_orm(string_value = "PREPROCESSOR_CONFIG")]
    PreprocessorConfig,
    #[sea_orm(string_value = "PROJECTION")]
    Projection,
    #[sea_orm(string_value = "PROJECT")]
    Project,
    #[sea_orm(string_value = "BOARD")]
    Board,
    #[sea_orm(string_value = "OTHER")]
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "LLMProvider")]
pub enum LlmProvider {
    #[sea_orm(string_value = "HuggingFace")]
    HuggingFace,
    #[sea_orm(string_value = "OpenAI")]
    OpenAi,
    #[sea_orm(string_value = "Anthropic")]
    Anthropic,
    #[sea_orm(string_value = "AzureOpenAI")]
    AzureOpenAi,
    #[sea_orm(string_value = "Google")]
    Google,
    #[sea_orm(string_value = "IBM")]
    Ibm,
    #[sea_orm(string_value = "X")]
    X,
    #[sea_orm(string_value = "Bedrock")]
    Bedrock,
    #[sea_orm(string_value = "Deepseek")]
    Deepseek,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "ProjectStatus")]
pub enum ProjectStatus {
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
    #[sea_orm(string_value = "ARCHIVED")]
    Archived,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "ProjectVisibility")]
pub enum ProjectVisibility {
    #[sea_orm(string_value = "PUBLIC")]
    Public,
    #[sea_orm(string_value = "PUBLIC_REQUEST_TO_JOIN")]
    PublicRequestToJoin,
    #[sea_orm(string_value = "PRIVATE")]
    Private,
    #[sea_orm(string_value = "PROTOTYPE")]
    Prototype,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "PublicationRequestStatus"
)]
pub enum PublicationRequestStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "ON_HOLD")]
    OnHold,
    #[sea_orm(string_value = "ACCEPTED")]
    Accepted,
    #[sea_orm(string_value = "REJECTED")]
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "SwimlaneSize")]
pub enum SwimlaneSize {
    #[sea_orm(string_value = "FULLSCREEN")]
    Fullscreen,
    #[sea_orm(string_value = "HALFSCREEN")]
    Halfscreen,
    #[sea_orm(string_value = "THIRDSCREEN")]
    Thirdscreen,
    #[sea_orm(string_value = "THIRDSCREEN_MULTIROW")]
    ThirdscreenMultirow,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "SwimlaneType")]
pub enum SwimlaneType {
    #[sea_orm(string_value = "PROJECT")]
    Project,
    #[sea_orm(string_value = "ARTICLE")]
    Article,
    #[sea_orm(string_value = "CHAT")]
    Chat,
    #[sea_orm(string_value = "COURSE")]
    Course,
    #[sea_orm(string_value = "QUERY")]
    Query,
}
