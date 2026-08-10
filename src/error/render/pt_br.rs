//! Portuguese renderings — stderr prose for a human reader.
//!
//! Carries no wire contract: the JSON `message` is always English. Knob names
//! (`max_body_bytes`) stay verbatim because the operator types them into
//! `config.toml`; they are identifiers, not words.

use crate::error::detail::ErrorDetail;
use crate::error::vocab::{
    AllowlistStage, DestructiveEffect, GenderPtBr, InternalOp, IoCause, IoOp, Subject,
};

impl Subject {
    /// Portuguese name used inside a rendered sentence.
    ///
    /// Knob names (`max_body_bytes`) stay verbatim: they are identifiers the
    /// operator types into `config.toml`, not words to be translated.
    pub(crate) fn pt_br(self) -> &'static str {
        match self {
            Self::CrateName => "nome da crate",
            Self::CrateRef => "referência de crate",
            Self::SearchQuery => "consulta de busca",
            Self::ItemPath => "caminho do item",
            Self::ItemPathSegment => "segmento de caminho de item",
            Self::Version => "versão",
            Self::PageToken => "page-token",
            Self::UserAgent => "user-agent",
            Self::Contact => "contato",
            Self::Origin => "URL de origem",
            Self::Timeout => "timeout",
            Self::ConnectTimeout => "connect_timeout",
            Self::MaxBodyBytes => "max_body_bytes",
            Self::MaxOutputBytes => "max_output_bytes",
            Self::ConfigFile => "config.toml",
            Self::LogDirective => "log_directive",
        }
    }
}

impl IoOp {
    pub(crate) fn pt_br(self) -> &'static str {
        match self {
            Self::Read => "ler",
            Self::Write => "escrever",
            Self::CreateDir => "criar diretório",
            Self::CreateTemp => "criar arquivo temporário",
            Self::Rename => "renomear",
            Self::Sync => "sincronizar",
            Self::Install => "instalar",
            Self::OpenLock => "abrir arquivo de lock",
            Self::Lock => "obter lock exclusivo",
            Self::ReadDir => "listar diretório",
            Self::Remove => "remover",
        }
    }
}

impl InternalOp {
    pub(crate) fn pt_br(self) -> &'static str {
        match self {
            Self::JsonSerialize => "falha ao serializar JSON",
            Self::JsonPrettyPrint => "falha ao formatar JSON",
            Self::StdoutWrite => "falha ao escrever em stdout",
            Self::EmbeddedSchemaInvalid => "o schema embutido não é JSON válido",
            Self::UrlBuild => "URL construída é inválida",
            Self::SemaphoreClosed => "semáforo de concorrência fechado",
            Self::WorkerJoin => "falha ao aguardar worker de CPU",
            Self::WorkerPanic => "worker de CPU entrou em pânico durante o parse",
            Self::ClockBeforeEpoch => "relógio do sistema anterior à época UNIX",
            Self::CacheKeyMalformed => {
                "chave de cache não é sha256 hex válido (invariante interna quebrada)"
            }
            Self::AssocPathTooShort => {
                "invariante de caminho de item associado quebrada: esperados ao menos 2 segmentos"
            }
            Self::AssocParentOwnsNoPage => "o kind pai do item associado não tem página no rustdoc",
            Self::AllowedOriginUnparseable => "a origem permitida não é uma URL válida",
            Self::SyntheticParseFailure => "falha de parse sintética",
        }
    }
}

impl AllowlistStage {
    pub(crate) fn pt_br(self) -> &'static str {
        match self {
            Self::Request => "host",
            Self::Redirect => "host de redirecionamento",
            Self::FinalUrl => "host da URL final",
        }
    }
}

/// Portuguese rendering — stderr prose only, never the JSON `message`.
pub(crate) fn render_pt_br(d: &ErrorDetail) -> String {
    match d {
        ErrorDetail::Empty { subject } => {
            let adj = match subject.gender_pt_br() {
                GenderPtBr::Masculine => "vazio",
                GenderPtBr::Feminine => "vazia",
            };
            format!("{} está {adj}", subject.pt_br())
        }
        ErrorDetail::TooLong { subject, limit } => {
            format!("{} excede {limit} caracteres", subject.pt_br())
        }
        ErrorDetail::Invalid { subject, value } => {
            let adj = match subject.gender_pt_br() {
                GenderPtBr::Masculine => "inválido",
                GenderPtBr::Feminine => "inválida",
            };
            format!("{} {adj}: '{value}'", subject.pt_br())
        }
        ErrorDetail::ControlCharacters { subject } => format!(
            "{} contém caracteres de controle ou invisíveis",
            subject.pt_br()
        ),
        ErrorDetail::NotVisibleAscii { subject } => format!(
            "{} deve ser ASCII visível (sem caracteres de controle nem não-ASCII)",
            subject.pt_br()
        ),
        ErrorDetail::ContainsWhitespace { subject, value } => {
            format!("{} contém espaço em branco: '{value}'", subject.pt_br())
        }
        ErrorDetail::MustBeAtLeastOneSecond { subject, .. } => {
            format!("{} deve ser >= 1 segundo (recebido 0)", subject.pt_br())
        }
        ErrorDetail::AboveHardMaximum {
            subject, hard_max, ..
        } => {
            format!("{} excede o máximo absoluto ({hard_max})", subject.pt_br())
        }

        ErrorDetail::ItemPathNoSegments => "caminho do item não tem segmentos".into(),
        ErrorDetail::ItemPathMissingItemName => "caminho do item sem nome do item".into(),
        ErrorDetail::CrateRefMultipleAt => {
            "referência de crate deve conter no máximo um '@'".into()
        }
        ErrorDetail::CrateRefEmptyName => "nome da crate está vazio antes do '@'".into(),
        ErrorDetail::CrateRefEmptyVersion => "versão está vazia após o '@'".into(),
        ErrorDetail::VersionBuildMetadata => "metadados de build da versão não são aceitos".into(),
        ErrorDetail::VersionVPrefix => "a versão não deve começar com o prefixo 'v'".into(),

        ErrorDetail::UnknownMatchMode { value } => {
            format!(
                "modo de correspondência desconhecido '{value}' (esperado exact|prefix|substring)"
            )
        }
        ErrorDetail::UnknownItemType { value } => {
            format!("tipo de item desconhecido '{value}'")
        }
        ErrorDetail::UnsupportedLang { tag } => {
            format!("idioma não suportado '{tag}'; esperado en ou pt-BR")
        }
        ErrorDetail::UnknownSchemaCommand { value } => {
            format!("comando de schema desconhecido '{value}'")
        }
        ErrorDetail::JsonFormatConflict => {
            "não é possível combinar --json com --format text ou --format markdown".into()
        }
        // Upstream text: see the variant docs for why this is not translated.
        ErrorDetail::ClapUsage { message } => message.clone(),
        ErrorDetail::InvalidFilterExpression { expr, reason } => {
            format!("expressão --filter inválida `{expr}`: {reason}")
        }
        ErrorDetail::ModuleFilterUnsupported => concat!(
            "search-in-crate --item-type module não é suportado ",
            "(all.html não tem índice de módulos); use get-item com kind module"
        )
        .into(),

        ErrorDetail::PageBelowOne => "page deve ser >= 1 (recebido 0 ou ausente)".into(),
        ErrorDetail::PerPageOutOfRange { max, got } => {
            format!("per_page deve estar em 1..={max} (recebido {got})")
        }

        ErrorDetail::OriginBadScheme { scheme } => {
            format!("o esquema da origem deve ser http ou https, recebido '{scheme}'")
        }
        ErrorDetail::OriginMissingHost { url } => {
            format!("a URL de origem deve incluir um host: '{url}'")
        }
        ErrorDetail::OriginNotAllowlisted { host, allowed } => format!(
            "host de origem fora da allowlist: '{host}' (permitidos: {allowed}; \
             loopback exige allow_loopback via CLI ou config.toml)"
        ),
        ErrorDetail::ItemPathSegmentCharset { segment } => format!(
            "segmento de caminho de item inválido '{segment}' (use letras, dígitos, sublinhado \
             ou hífen; hifens viram sublinhado; separe com :: ou /)"
        ),
        ErrorDetail::ConflictingVersions {
            from_ref,
            from_flag,
        } => format!("versões conflitantes: crate@{from_ref} vs --crate-version {from_flag}"),
        ErrorDetail::ConfigDirUnresolved => concat!(
            "não foi possível resolver o diretório de configuração ",
            "(defina --config-dir ou garanta o XDG config home)"
        )
        .into(),
        ErrorDetail::CacheDirUnresolved => concat!(
            "não foi possível resolver o diretório de cache ",
            "(defina --cache-dir ou garanta o XDG cache home)"
        )
        .into(),
        ErrorDetail::AmbientTargetRefused {
            verb,
            target,
            target_flag,
            waiver_flag,
            effect,
        } => {
            let harm = match effect {
                DestructiveEffect::Delete => "apagaria",
                DestructiveEffect::Overwrite => "sobrescreveria",
            };
            format!(
                "{verb} {harm} um alvo de ambiente que nunca lhe foi dado: {target}; \
                 nomeie-o no argv com {target_flag} <DIR>, ou aceite-o com {waiver_flag}"
            )
        }
        ErrorDetail::ConfigTomlInvalid => "config.toml inválido".into(),
        ErrorDetail::ConfigTomlNotUtf8 => "config.toml não é UTF-8 válido".into(),
        ErrorDetail::ConfigTomlTooLarge { max_bytes } => {
            format!("config.toml excede o tamanho máximo ({max_bytes} bytes)")
        }
        ErrorDetail::ConfigAlreadyExists { path } => {
            format!("a configuração já existe: {path} (use --force para sobrescrever)")
        }
        ErrorDetail::UserAgentHeaderInvalid => "cabeçalho user-agent inválido".into(),

        ErrorDetail::HttpStatus { status, hint } => match status {
            400 => format!("requisição inválida no remoto: {hint}"),
            404 => format!("recurso não encontrado: {hint}"),
            408 => format!("tempo esgotado no remoto (HTTP 408): {hint}"),
            429 => format!("limitado por taxa no remoto: {hint}"),
            500 | 502 | 503 | 504 => format!("remoto indisponível (HTTP {status}): {hint}"),
            other => format!("status HTTP inesperado {other}: {hint}"),
        },
        ErrorDetail::HostNotAllowlisted { host, stage } => {
            format!("{} fora da allowlist: {host}", stage.pt_br())
        }
        ErrorDetail::HttpRequestFailed { url, .. } => {
            format!("a requisição HTTP falhou para {url}")
        }
        ErrorDetail::RedirectLimitExceeded => "limite de redirecionamentos excedido".into(),
        ErrorDetail::HttpClientBuild => "falha ao construir o cliente HTTP".into(),
        ErrorDetail::BodyRead => "falha ao ler o corpo da resposta".into(),
        ErrorDetail::BodyOverBudget { max_bytes } => {
            format!("corpo da resposta excede max_body_bytes ({max_bytes})")
        }
        ErrorDetail::CachedBodyOverBudget { max_bytes } => {
            format!("corpo em cache excede max_body_bytes ({max_bytes})")
        }
        ErrorDetail::BodyReserveFailed { bytes } => {
            format!("falha ao reservar {bytes} bytes para o corpo da resposta")
        }
        ErrorDetail::BodyNotUtf8 => "corpo da resposta não é UTF-8 válido".into(),
        ErrorDetail::UnexpectedContentType { expected, got } => format!(
            "Content-Type inesperado para resposta {}: {got}",
            expected.label()
        ),
        ErrorDetail::OutputOverBudget => "corpo grande demais".into(),

        ErrorDetail::HtmlToMarkdown => "falha ao converter HTML para Markdown".into(),
        ErrorDetail::CratesIoJson => "falha ao parsear o JSON do crates.io".into(),
        ErrorDetail::AssocParentPageNotFound => "página do tipo pai não encontrada".into(),
        ErrorDetail::AssocAnchorMissing { anchors } => {
            format!("âncora de item associado não encontrada: {anchors}")
        }
        ErrorDetail::AssocAnchorEmpty { anchor_id } => {
            format!("âncora de item associado vazia: {anchor_id}")
        }
        ErrorDetail::MemberKindNeedsParent {
            kind,
            member,
            parent_kinds,
        } => format!(
            "{kind} não tem página própria: qualifique como Pai::{member} (kinds de pai: {parent_kinds})"
        ),
        ErrorDetail::HitJoinFailed => {
            "falha ao resolver o href (fora da origem ou inválido contra a base source_url)".into()
        }
        ErrorDetail::HitBaseInvalid => "source_url inválida como base de resolução".into(),

        ErrorDetail::Interrupted => "interrompido por SIGINT".into(),
        ErrorDetail::Terminated => "encerrado por SIGTERM".into(),
        ErrorDetail::BrokenPipe => "pipe quebrado ao escrever em stdout".into(),
        ErrorDetail::WallClockTimeout { secs } => format!("tempo esgotado após {secs}s"),

        ErrorDetail::Io { op, path, cause } => {
            let hint = match cause {
                IoCause::Transient => {
                    " (transitório; o mesmo comando pode funcionar quando a condição passar)"
                }
                IoCause::Permanent => " (permanente; o ambiente precisa mudar antes)",
            };
            match path {
                Some(p) => format!("falha ao {} {p}{hint}", op.pt_br()),
                None => format!("falha ao {}{hint}", op.pt_br()),
            }
        }
        ErrorDetail::Internal { op } => op.pt_br().into(),

        ErrorDetail::WithSuggestions { base, suggestions } => {
            let list = crate::error::suggestion::join_suggestions(suggestions);
            format!("{}; sugestões: {list}", render_pt_br(base))
        }
    }
}
