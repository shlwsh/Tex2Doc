# Paper2/Paper3 journal profile verification summary

- Generated at: 2026-06-22T23:08:51.5806433+08:00
- Output dir: `examples/output/journal-profile-generalization`
- Matrix: 14 cases; generated 14; failed 0; A/B 11
- Grade counts: A=5, B=6, C=0, D=3, F=0

| Paper | Requested profile | Effective profile | Backend | Score | Bytes | Para | Tables | Media | OMML | Raw hits | Unresolved | Grade | Issues |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| paper2 | generic | generic-article | RuleBased | 94 | 39061 | 793 | 19 | 0 | 225 | 4 | 0 | A | no-media-files, requested-effective-profile-mismatch |
| paper2 | jos-paper |  |  | 0 | 21474 | 282 | 8 | 0 | 73 | 4 | 0 | D | command-exit-7, no-media-files, compatibility-below-plan-threshold, requested-effective-profile-mismatch |
| paper2 | tacl | tacl | RuleBased | 82 | 39061 | 793 | 19 | 0 | 225 | 4 | 0 | A | no-media-files |
| paper2 | cvpr | cvpr | RuleBased | 88 | 18680 | 282 | 8 | 0 | 73 | 4 | 0 | D | docx-too-small, no-media-files |
| paper2 | nature | nature | RuleBased | 82 | 39065 | 793 | 19 | 0 | 225 | 4 | 0 | A | no-media-files |
| paper2 | springer | springer | RuleBased | 82 | 39062 | 793 | 19 | 0 | 225 | 4 | 0 | A | no-media-files |
| paper2 | chinese-academic | chinese-academic | RuleBased | 88 | 38415 | 794 | 21 | 0 | 177 | 2 | 0 | A | no-media-files |
| paper3 | generic | generic-article | RuleBased | 76 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | requested-effective-profile-mismatch |
| paper3 | jos-paper | jos-paper | RuleBased | 64 | 3057672 | 650 | 11 | 10 | 192 | 0 | 0 | B | compatibility-below-plan-threshold |
| paper3 | tacl | tacl | RuleBased | 64 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | compatibility-below-plan-threshold |
| paper3 | cvpr | cvpr | RuleBased | 64 | 3054778 | 650 | 11 | 10 | 192 | 0 | 0 | B | compatibility-below-plan-threshold |
| paper3 | nature | nature | RuleBased | 64 | 3054782 | 650 | 11 | 10 | 192 | 0 | 0 | B | compatibility-below-plan-threshold |
| paper3 | springer |  |  | 0 | 3054777 | 650 | 11 | 10 | 192 | 0 | 0 | D | command-exit-7, compatibility-below-plan-threshold, requested-effective-profile-mismatch |
| paper3 | chinese-academic | chinese-academic | RuleBased | 70 | 3053677 | 632 | 11 | 10 | 200 | 2 | 0 | B |  |

## Notes

- Word/WPS/LibreOffice visual opening was not automated; snapshots record package-level openability and header text extracted from DOCX XML.
- `compatibility-below-plan-threshold` follows the plan threshold of 70; preview quality was used so reports are still emitted for borderline cases.
- Profile-specific wrapper generation is represented by copied entry files plus `inputs/manifest.json`; full template-shell rewriting remains a follow-up task.

## Application Open Validation

- Word COM: 0/14 passed. All cases failed with `文件可能已经损坏。` using `C:/Program Files/Microsoft Office/root/Office16/WINWORD.EXE` 16.0.20026.20182.
- LibreOffice headless: 14/14 passed. All cases converted to PDF using `C:/Program Files/LibreOffice/program/soffice.com` 26.2.4.2.
- PDF outputs: `examples/output/journal-profile-generalization/soffice-pdf/`.
- Interpretation: DOCX packages are readable by LibreOffice but currently fail Microsoft Word strict OOXML open validation, so GA/paid-beta quality gates remain blocked until Word compatibility is fixed.
