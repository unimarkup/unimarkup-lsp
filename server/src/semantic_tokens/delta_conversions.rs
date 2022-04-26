
/// Function converts a Unimarkup line number to LSP by starting at 0
pub(crate) fn to_lsp_line_nr(line_nr: usize) -> u32 {
	let nr = line_nr as u32;
	nr - 1
}
