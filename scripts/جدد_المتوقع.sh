#!/usr/bin/env bash
#
# يجدّد الخرج المتوقع لكل مثال في examples/متوقع/
# Regenerate the committed expected output for every example.
#
# اقرأ الفرق قبل الإيداع: هذا الملف هو ما يمنع قيمة خاطئة من المرور، فتجديده
# بلا قراءة يحوّله من فحص إلى ختم.
# Read the diff before committing. These files are what stop a wrong value from
# passing CI; regenerating without reading turns the check into a rubber stamp.

set -euo pipefail

cd "$(dirname "$0")/.."

TARQEEM="${TARQEEM:-./target/release/tarqeem}"

if [ ! -x "$TARQEEM" ]; then
    echo "لم يُبنَ المترجم بعد / compiler not built: $TARQEEM" >&2
    echo "  cargo build --release" >&2
    exit 1
fi

# TARQEEM_HOME يحجب مكتبة المستودع القياسية، فيُلغى هنا كما تفعل التكاملة.
unset TARQEEM_HOME

mkdir -p examples/متوقع

for file in examples/*.ترقيم; do
    name="$(basename "$file" .ترقيم)"
    out="examples/متوقع/${name}.خرج"

    if ! "$TARQEEM" run "$file" > "$out" 2>&1; then
        echo "فشل تنفيذ ${name} / ${name} failed to run — الخرج المحفوظ يحمل الخطأ" >&2
        exit 1
    fi

    printf '%-24s %4s سطر\n' "$name" "$(wc -l < "$out" | tr -d ' ')"
done

echo
echo "راجع الفرق قبل الإيداع / review the diff before committing:"
echo "  git diff examples/متوقع/"
