"""Audit the fixed denominator and reported counts; not a substitute for QA."""
from pathlib import Path
rows = [line.split('|') for line in (Path(__file__).resolve().parent.parent/'ALIGNMENT.md').read_text(encoding='utf-8').splitlines() if line.startswith('| F')]
assert [row[1].strip() for row in rows] == [f'F{i:02}' for i in range(1,37)], 'Fixed 36 capability IDs changed'
valid = {'完成', '部分', '缺少', '未驗證'}
assert all(row[4].strip() in valid and row[5].strip() in valid and row[6].strip() in valid and row[7].strip() in valid for row in rows)
before = sum(row[4].strip() == '完成' for row in rows)
after = sum(row[9].strip() == '完成' for row in rows)
assert before == 11 and after == 28, 'Reported counts do not match matrix'
print(f'Fixed core: {before}/36 -> {after}/36 ({after/36:.1%}); not full-product parity')
