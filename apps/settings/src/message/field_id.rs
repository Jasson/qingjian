//! 输入框的编号：编辑状态按它存在 `Settings::fields` 里。

/// 输入框：云服务、本地实时翻译、短语编辑与生词本搜索。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    BaseUrl,
    Model,
    ApiKey,
    LiveBaseUrl,
    LiveModel,
    SpeechBaseUrl,
    SpeechVoice,
    LiveDraftExcludedApps,
    PhraseCode,
    PhraseText,
    /// 生词本页的搜索框，不存进配置。
    VocabularySearch,
}

impl FieldId {
    /// 存进配置的那几个（短语的两个随卡片一起存）。
    pub const ALL: [Self; 8] = [
        Self::BaseUrl,
        Self::Model,
        Self::ApiKey,
        Self::LiveBaseUrl,
        Self::LiveModel,
        Self::SpeechBaseUrl,
        Self::SpeechVoice,
        Self::LiveDraftExcludedApps,
    ];

    /// 对应配置分节里的键。
    pub fn key(self) -> &'static str {
        match self {
            Self::BaseUrl | Self::LiveBaseUrl | Self::SpeechBaseUrl => "base_url",
            Self::Model | Self::LiveModel => "model",
            Self::SpeechVoice => "voice",
            Self::ApiKey => "api_key",
            Self::LiveDraftExcludedApps => "draft_excluded_apps",
            Self::PhraseCode => "code",
            Self::PhraseText => "text",
            Self::VocabularySearch => "vocabulary-search",
        }
    }
}
