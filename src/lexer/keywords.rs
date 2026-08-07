//! Keyword mappings for Arabic keywords
//!
//! Tarqeem is an Arabic-only programming language. English keywords are not supported.

use super::TokenKind;
use phf::phf_map;

pub static KEYWORDS: phf::Map<&'static str, TokenKind> = phf_map! {
    "متغير" => TokenKind::Let,
    "ثابت" => TokenKind::Const,

    "دالة" => TokenKind::Function,
    "أرجع" => TokenKind::Return,
    "ارجع" => TokenKind::Return,  // Without hamza variant
    "متوازي" => TokenKind::Async,  // دالة متوازية (غير متزامنة)
    "انتظر" => TokenKind::Await,

    "إذا" => TokenKind::If,
    "اذا" => TokenKind::If,  // Without hamza variant
    "وإلا" => TokenKind::Else,
    "والا" => TokenKind::Else,  // Without hamza variant
    "تطابق" => TokenKind::Match,
    "حالة" => TokenKind::Case,
    "غير_ذلك" => TokenKind::Default,

    "طالما" => TokenKind::While,
    "لكل" => TokenKind::For,
    "في" => TokenKind::In,
    "افعل" => TokenKind::Do,
    "أوقف" => TokenKind::Break,
    "اوقف" => TokenKind::Break,  // Without hamza variant
    "استمر" => TokenKind::Continue,

    "صنف" => TokenKind::Class,
    "ميثاق" => TokenKind::Interface,  // عقد يُلزم الصنف بتنفيذ دوال معينة
    "تعداد" => TokenKind::Enum,       // مجموعة من القيم المسماة المحددة مسبقاً
    "يرث" => TokenKind::Extends,
    "يلتزم" => TokenKind::Implements,  // الصنف يلتزم بتنفيذ الميثاق
    "عام" => TokenKind::Public,
    "خاص" => TokenKind::Private,
    "محمي" => TokenKind::Protected,
    "مشترك" => TokenKind::Static,  // عضو مشترك بين جميع نسخ الصنف
    "منشئ" => TokenKind::Constructor,
    "هذا" => TokenKind::This,
    "الأصل" => TokenKind::Super,  // مرجع الصنف الأب في الوراثة
    "الاصل" => TokenKind::Super,  // Without hamza variant
    "جديد" => TokenKind::New,

    "خاصية" => TokenKind::Property,  // تعريف خاصية مع وصول محكوم
    "احصل" => TokenKind::Get,        // قارئ الخاصية
    "عيّن" => TokenKind::Set,        // كاتب الخاصية
    "عين" => TokenKind::Set,         // Without shadda variant

    "حاول" => TokenKind::Try,
    "التقط" => TokenKind::Catch,
    "أخيراً" => TokenKind::Finally,
    "اخيرا" => TokenKind::Finally,  // Without hamza variant
    "ارمِ" => TokenKind::Throw,
    "ارم" => TokenKind::Throw,  // Without kasra variant

    "استورد" => TokenKind::Import,
    "صدّر" => TokenKind::Export,
    "صدر" => TokenKind::Export,  // Without shadda variant
    "من" => TokenKind::From,
    "كـ" => TokenKind::As,
    "ك" => TokenKind::As,  // Without tatweel variant

    "صحيح" => TokenKind::True,
    "خطأ" => TokenKind::False,
    "خطا" => TokenKind::False,  // Without hamza variant
    "لا_شيء" => TokenKind::Null,  // قيمة فارغة

    "و" => TokenKind::And,
    "أو" => TokenKind::Or,
    "او" => TokenKind::Or,  // Without hamza variant
    "ليس" => TokenKind::Bang,

    "عدد" => TokenKind::TypeInt,
    "عدد_عشري" => TokenKind::TypeFloat,
    "نص" => TokenKind::TypeString,
    "منطقي" => TokenKind::TypeBool,
    "مصفوفة" => TokenKind::TypeArray,
    "قاموس" => TokenKind::TypeMap,
    "أي" => TokenKind::TypeAny,
    "اي" => TokenKind::TypeAny,  // Without hamza variant

    "بسم_الله" => TokenKind::Bismillah,
    "الحمد_لله" => TokenKind::Alhamdulillah,
};

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
    fn test_arabic_keywords_without_hamza() {
        assert_eq!(lookup_keyword("اذا"), Some(TokenKind::If));
        assert_eq!(lookup_keyword("والا"), Some(TokenKind::Else));
        assert_eq!(lookup_keyword("ارجع"), Some(TokenKind::Return));
        assert_eq!(lookup_keyword("الاصل"), Some(TokenKind::Super));
        assert_eq!(lookup_keyword("خطا"), Some(TokenKind::False));
    }

    #[test]
    fn test_english_keywords_not_supported() {
        assert_eq!(lookup_keyword("let"), None);
        assert_eq!(lookup_keyword("const"), None);
        assert_eq!(lookup_keyword("function"), None);
        assert_eq!(lookup_keyword("if"), None);
        assert_eq!(lookup_keyword("else"), None);
        assert_eq!(lookup_keyword("while"), None);
        assert_eq!(lookup_keyword("for"), None);
        assert_eq!(lookup_keyword("class"), None);
        assert_eq!(lookup_keyword("true"), None);
        assert_eq!(lookup_keyword("false"), None);
    }

    #[test]
    fn test_non_keyword() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("مرحبا"), None);
    }

    #[test]
    fn test_file_markers() {
        assert_eq!(lookup_keyword("بسم_الله"), Some(TokenKind::Bismillah));
        assert_eq!(lookup_keyword("الحمد_لله"), Some(TokenKind::Alhamdulillah));
    }

    #[test]
    fn test_type_keywords() {
        assert_eq!(lookup_keyword("عدد"), Some(TokenKind::TypeInt));
        assert_eq!(lookup_keyword("عدد_عشري"), Some(TokenKind::TypeFloat));
        assert_eq!(lookup_keyword("نص"), Some(TokenKind::TypeString));
        assert_eq!(lookup_keyword("منطقي"), Some(TokenKind::TypeBool));
        assert_eq!(lookup_keyword("مصفوفة"), Some(TokenKind::TypeArray));
        assert_eq!(lookup_keyword("قاموس"), Some(TokenKind::TypeMap));
        assert_eq!(lookup_keyword("أي"), Some(TokenKind::TypeAny));
    }

    #[test]
    fn test_oop_keywords() {
        assert_eq!(lookup_keyword("صنف"), Some(TokenKind::Class));
        assert_eq!(lookup_keyword("ميثاق"), Some(TokenKind::Interface)); // ميثاق بدلاً من واجهة
        assert_eq!(lookup_keyword("تعداد"), Some(TokenKind::Enum)); // تعداد للقيم المسماة
        assert_eq!(lookup_keyword("يرث"), Some(TokenKind::Extends));
        assert_eq!(lookup_keyword("يلتزم"), Some(TokenKind::Implements)); // يلتزم بدلاً من يطبق
        assert_eq!(lookup_keyword("عام"), Some(TokenKind::Public));
        assert_eq!(lookup_keyword("خاص"), Some(TokenKind::Private));
        assert_eq!(lookup_keyword("محمي"), Some(TokenKind::Protected));
        assert_eq!(lookup_keyword("مشترك"), Some(TokenKind::Static)); // مشترك بدلاً من ثابت_صنف
        assert_eq!(lookup_keyword("منشئ"), Some(TokenKind::Constructor));
        assert_eq!(lookup_keyword("هذا"), Some(TokenKind::This));
        assert_eq!(lookup_keyword("جديد"), Some(TokenKind::New));
    }

    #[test]
    fn test_old_keywords_not_supported() {
        assert_eq!(lookup_keyword("واجهة"), None);
        assert_eq!(lookup_keyword("يطبق"), None);
        assert_eq!(lookup_keyword("ثابت_صنف"), None);
        assert_eq!(lookup_keyword("فراغ"), None);
        assert_eq!(lookup_keyword("أساس"), None); // Now الأصل
        assert_eq!(lookup_keyword("عدم"), None); // Now لا_شيء
        assert_eq!(lookup_keyword("غير_متزامن"), None); // Now متوازي
    }

    #[test]
    fn test_property_accessor_keywords() {
        assert_eq!(lookup_keyword("خاصية"), Some(TokenKind::Property));
        assert_eq!(lookup_keyword("احصل"), Some(TokenKind::Get));
        assert_eq!(lookup_keyword("عيّن"), Some(TokenKind::Set));
        assert_eq!(lookup_keyword("عين"), Some(TokenKind::Set)); // Without shadda variant
    }
}
