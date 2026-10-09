"""Audit fixed IDs, historical status columns and counts; not a QA substitute."""
from pathlib import Path
rows = [line.split('|') for line in (Path(__file__).resolve().parent.parent/'ALIGNMENT.md').read_text(encoding='utf-8').splitlines() if line.startswith('| F')]
assert [row[1].strip() for row in rows] == [f'F{i:02}' for i in range(1,37)], 'Fixed 36 capability IDs changed'
valid = {'完成', '部分', '缺少', '未驗證'}
assert all(len(row)==12 and all(cell.strip() in valid for cell in row[4:11]) for row in rows)
counts = [sum(row[column].strip()=='完成' for row in rows) for column in range(4,11)]
assert counts == [11,21,23,25,27,28,29], 'Historical or latest counts changed'
print(f'Fixed core: {counts[0]}/36 -> {counts[-1]}/36 ({counts[-1]/36:.1%}); not full-product parity')
