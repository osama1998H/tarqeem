# Standard Library (stdlib_trq)

<div dir="rtl" align="right">

## المكتبة القياسية

هذا المجلد يحتوي على المكتبة القياسية لترقيم المكتوبة بلغة ترقيم نفسها.

### الوحدات

| الوحدة | الوصف |
|--------|-------|
| `مجموعات.trq` | مجموعات البيانات (قائمة، قاموس، مجموعة) |
| `رياضيات.trq` | دوال رياضية وثوابت |
| `نص.trq` | معالجة النصوص |
| `ملفات.trq` | عمليات الملفات |

### الاستخدام

```tarqeem
استورد { قائمة } من "مجموعات"
استورد { باي، مطلق } من "رياضيات"
استورد { فارغ } من "نص"

متغير أرقام = جديد قائمة<عدد>()
أرقام.أضف(مطلق(-5))
اطبع(باي)
```

</div>

## Standard Library

This directory contains the Tarqeem standard library written in Tarqeem itself.

### Modules

| Module | Description |
|--------|-------------|
| `مجموعات.trq` | Data collections (List, Map, Set) |
| `رياضيات.trq` | Math functions and constants |
| `نص.trq` | String processing utilities |
| `ملفات.trq` | File operations |

### Usage

```tarqeem
استورد { قائمة } من "مجموعات"
استورد { باي، مطلق } من "رياضيات"
استورد { فارغ } من "نص"

متغير numbers = جديد قائمة<عدد>()
numbers.أضف(مطلق(-5))
اطبع(باي)
```

### Development Status

These modules provide skeleton implementations. Many features depend on runtime bindings that are still in development.
