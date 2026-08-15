//! Resident 守护进程的 IPC 协议。
//!
//! 消息以 JSON 格式序列化，每行一条消息（换行分隔的 JSON）。
//! 每个请求是一行 JSON；每个响应也是一行 JSON。

use serde::{Deserialize, Serialize};

/// 文档会话的唯一标识符。
pub type SessionId = u64;

/// 页面范围选择规格。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageRange {
    /// 起始页（0 起始，包含）。
    pub start: usize,
    /// 结束页（0 起始，不包含）。`None` 表示"直到末尾"。
    pub end: Option<usize>,
}

/// 客户端发送给服务器的请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// 打开一个 PDF 文档并创建新会话。
    Open {
        /// PDF 文件路径。
        path: String,
        /// 打开模式（只读或读写）。
        mode: OpenMode,
    },
    /// 从页面中提取文本。
    ExtractText {
        /// 目标会话。
        session_id: SessionId,
        /// 页面范围（0 起始）。`None` = 所有页面。
        pages: Option<PageRange>,
    },
    /// 提取文档元数据。
    ExtractMetadata {
        /// 目标会话。
        session_id: SessionId,
    },
    /// 获取总页数。
    PageCount {
        /// 目标会话。
        session_id: SessionId,
    },
    /// 旋转某个页面。
    RotatePage {
        /// 目标会话。
        session_id: SessionId,
        /// 页码（1 起始，匹配 `PdfManipulator` 约定）。
        page: usize,
        /// 旋转角度：0、90、180、270 度顺时针。
        rotation: u16,
    },
    /// 保存文档。
    Save {
        /// 目标会话。
        session_id: SessionId,
        /// 可选的输出路径。`None` = 保存到原始路径。
        path: Option<String>,
    },
    /// 关闭会话并释放资源。
    Close {
        /// 目标会话。
        session_id: SessionId,
    },
    /// 存活探测。
    Ping,
    /// 优雅关闭服务器。
    Shutdown,
}

/// 服务器返回给客户端的响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// 请求是否成功。
    pub ok: bool,
    /// 会话 ID（当响应是会话级别时存在）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// 响应载荷（成功时存在）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ResponseData>,
    /// 错误码（失败时存在）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 错误消息（失败时存在）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// 成功响应的载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResponseData {
    /// 文本提取结果。
    Text {
        /// 提取的文本内容。
        content: String,
    },
    /// 文档元数据。
    Metadata {
        /// 文档元数据。
        #[serde(flatten)]
        metadata: PdfMetadataDto,
    },
    /// 页数。
    PageCount {
        /// 页面数量。
        count: usize,
    },
    /// 文档保存成功。
    Saved {
        /// 文档保存到的路径。
        path: String,
    },
    /// Pong 响应。
    Pong,
    /// 空确认。
    None,
}

/// 用于 IPC 序列化的元数据 DTO。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfMetadataDto {
    /// 文档标题。
    pub title: Option<String>,
    /// 文档作者。
    pub author: Option<String>,
    /// 文档主题。
    pub subject: Option<String>,
    /// 文档关键词。
    pub keywords: Option<String>,
    /// 创建应用程序。
    pub creator: Option<String>,
    /// 生产应用程序。
    pub producer: Option<String>,
}

/// 文档会话的打开模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenMode {
    /// 只读访问（不允许修改）。
    ReadOnly,
    /// 读写访问（允许修改和保存）。
    ReadWrite,
}

// --- Response 构造方法 ---

impl Response {
    /// 创建一个无载荷的成功响应。
    #[must_use]
    pub fn ok() -> Self {
        Self {
            ok: true,
            session_id: None,
            data: None,
            error_code: None,
            error_message: None,
        }
    }

    /// 创建一个带会话 ID 的成功响应。
    #[must_use]
    pub fn ok_session(session_id: SessionId) -> Self {
        Self {
            ok: true,
            session_id: Some(session_id),
            data: None,
            error_code: None,
            error_message: None,
        }
    }

    /// 创建一个带数据的成功响应。
    #[must_use]
    pub fn ok_data(session_id: Option<SessionId>, data: ResponseData) -> Self {
        Self {
            ok: true,
            session_id,
            data: Some(data),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建一个错误响应。
    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            session_id: None,
            data: None,
            error_code: Some(code.into()),
            error_message: Some(message.into()),
        }
    }
}

/// 最大消息大小（1 MB），防止畸形客户端导致内存耗尽。
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
