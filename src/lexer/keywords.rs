//! Keyword mappings for Arabic and English keywords

use super::TokenKind;
use phf::phf_map;

/// Static map of keywords to token kinds
/// Includes both Arabic and English keywords
pub static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    // ============ Variables ============
    "متغير" => TokenKind::Let,
    "let" => TokenKind::Let,
    "var" => TokenKind::Let,
    "ثابت" => TokenKind::Const,
    "const" => TokenKind::Const,

    // ============ Functions ============
    "دالة" => TokenKind::Function,
    "function" => TokenKind::Function,
    "fn" => TokenKind::Function,
    "أرجع" => TokenKind::Return,
    "ارجع" => TokenKind::Return,
    "return" => TokenKind::Return,
    "غير_متزامن" => TokenKind::Async,
    "async" => TokenKind::Async,
    "انتظر" => TokenKind::Await,
    "await" => TokenKind::Await,

    // ============ Control Flow ============
    "إذا" => TokenKind::If,
    "اذا" => TokenKind::If,
    "if" => TokenKind::If,
    "وإلا" => TokenKind::Else,
    "والا" => TokenKind::Else,
    "else" => TokenKind::Else,
    "تطابق" => TokenKind::Match,
    "match" => TokenKind::Match,
    "switch" => TokenKind::Match,
    "حالة" => TokenKind::Case,
    "case" => TokenKind::Case,
    "غير_ذلك" => TokenKind::Default,
    "default" => TokenKind::Default,

    // ============ Loops ============
    "طالما" => TokenKind::While,
    "while" => TokenKind::While,
    "لكل" => TokenKind::For,
    "for" => TokenKind::For,
    "في" => TokenKind::In,
    "in" => TokenKind::In,
    "افعل" => TokenKind::Do,
    "do" => TokenKind::Do,
    "أوقف" => TokenKind::Break,
    "اوقف" => TokenKind::Break,
    "break" => TokenKind::Break,
    "استمر" => TokenKind::Continue,
    "continue" => TokenKind::Continue,

    // ============ OOP ============
    "صنف" => TokenKind::Class,
    "class" => TokenKind::Class,
    "واجهة" => TokenKind::Interface,
    "interface" => TokenKind::Interface,
    "يرث" => TokenKind::Extends,
    "extends" => TokenKind::Extends,
    "يطبق" => TokenKind::Implements,
    "implements" => TokenKind::Implements,
    "عام" => TokenKind::Public,
    "public" => TokenKind::Public,
    "خاص" => TokenKind::Private,
    "private" => TokenKind::Private,
    "محمي" => TokenKind::Protected,
    "protected" => TokenKind::Protected,
    "ثابت_صنف" => TokenKind::Static,
    "static" => TokenKind::Static,
    "منشئ" => TokenKind::Constructor,
    "constructor" => TokenKind::Constructor,
    "هذا" => TokenKind::This,
    "this" => TokenKind::This,
    "أساس" => TokenKind::Super,
    "اساس" => TokenKind::Super,
    "super" => TokenKind::Super,
    "جديد" => TokenKind::New,
    "new" => TokenKind::New,

    // ============ Error Handling ============
    "حاول" => TokenKind::Try,
    "try" => TokenKind::Try,
    "التقط" => TokenKind::Catch,
    "catch" => TokenKind::Catch,
    "أخيراً" => TokenKind::Finally,
    "اخيرا" => TokenKind::Finally,
    "finally" => TokenKind::Finally,
    "ارمِ" => TokenKind::Throw,
    "ارم" => TokenKind::Throw,
    "throw" => TokenKind::Throw,

    // ============ Modules ============
    "استورد" => TokenKind::Import,
    "import" => TokenKind::Import,
    "صدّر" => TokenKind::Export,
    "صدر" => TokenKind::Export,
    "export" => TokenKind::Export,
    "من" => TokenKind::From,
    "from" => TokenKind::From,
    "كـ" => TokenKind::As,
    "ك" => TokenKind::As,
    "as" => TokenKind::As,

    // ============ Boolean/Null ============
    "صحيح" => TokenKind::True,
    "true" => TokenKind::True,
    "خطأ" => TokenKind::False,
    "خطا" => TokenKind::False,
    "false" => TokenKind::False,
    "عدم" => TokenKind::Null,
    "null" => TokenKind::Null,
    "none" => TokenKind::Null,

    // ============ Logical Operators (Arabic words) ============
    "و" => TokenKind::And,
    "أو" => TokenKind::Or,
    "او" => TokenKind::Or,
    "ليس" => TokenKind::Bang,
    "not" => TokenKind::Bang,

    // ============ Type Keywords ============
    "عدد" => TokenKind::TypeInt,
    "int" => TokenKind::TypeInt,
    "عدد_عشري" => TokenKind::TypeFloat,
    "float" => TokenKind::TypeFloat,
    "نص" => TokenKind::TypeString,
    "string" => TokenKind::TypeString,
    "منطقي" => TokenKind::TypeBool,
    "bool" => TokenKind::TypeBool,
    "مصفوفة" => TokenKind::TypeArray,
    "array" => TokenKind::TypeArray,
    "قاموس" => TokenKind::TypeMap,
    "map" => TokenKind::TypeMap,
    "dict" => TokenKind::TypeMap,
    "فراغ" => TokenKind::TypeVoid,
    "void" => TokenKind::TypeVoid,
    "أي" => TokenKind::TypeAny,
    "اي" => TokenKind::TypeAny,
    "any" => TokenKind::TypeAny,

    // ============ File Markers ============
    "بسم_الله" => TokenKind::Bismillah,
    "bismillah" => TokenKind::Bismillah,
    "الحمد_لله" => TokenKind::Alhamdulillah,
    "alhamdulillah" => TokenKind::Alhamdulillah,
};

/// Look up a keyword in the keyword map
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    KEYWORDS.get(ident).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arabic_keywords() {
        assert_eq!(lookup_keyword("متغير"), Some(TokenKind::Let));
        assert_eq!(lookup_keyword("ثابت"), Some(TokenKind::Const));
        assert_eq!(lookup_keyword("دالة"), Some(TokenKind::Function));
        assert_eq!(lookup_keyword("إذا"), Some(TokenKind::If));
        assert_eq!(lookup_keyword("صحيح"), Some(TokenKind::True));
    }

    #[test]
    fn test_english_keywords() {
        assert_eq!(lookup_keyword("let"), Some(TokenKind::Let));
        assert_eq!(lookup_keyword("const"), Some(TokenKind::Const));
        assert_eq!(lookup_keyword("function"), Some(TokenKind::Function));
        assert_eq!(lookup_keyword("if"), Some(TokenKind::If));
        assert_eq!(lookup_keyword("true"), Some(TokenKind::True));
    }

    #[test]
    fn test_non_keyword() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("مرحبا"), None);
    }

    #[test]
    fn test_file_markers() {
        assert_eq!(lookup_keyword("بسم_الله"), Some(TokenKind::Bismillah));
        assert_eq!(lookup_keyword("bismillah"), Some(TokenKind::Bismillah));
        assert_eq!(lookup_keyword("الحمد_لله"), Some(TokenKind::Alhamdulillah));
        assert_eq!(
            lookup_keyword("alhamdulillah"),
            Some(TokenKind::Alhamdulillah)
        );
    }
}
