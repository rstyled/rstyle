# Frozen inventory snapshot

`historical-selection.json` copies `eval/hir-coverage/selection.json` at commit
`1d1cb4cf`. The companion oracle checks the selected function against entry 11
without reading the evaluation directory. The copied inventory, existing source
identities and oracle checks are unchanged.
