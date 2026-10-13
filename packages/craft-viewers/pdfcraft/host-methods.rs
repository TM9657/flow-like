impl PdfCraftApp {
    pub fn host_revision(&self) -> u64 {
        self.active_ids()
            .and_then(|(_, id)| self.session.get(id))
            .map(|doc| doc.edit_generation())
            .unwrap_or(0)
    }

    pub fn host_export(&mut self) -> Result<(u64, std::sync::Arc<Vec<u8>>), String> {
        let (_, id) = self.active_ids().ok_or("Open a PDF first.")?;
        if !self.commit_typing_in(id) || !self.flatten_fill_sign_for_save(id) {
            return Err("Finish the current edit before saving.".into());
        }
        let bytes = self.session.save_bytes(id).map_err(|e| e.to_string())?;
        Ok((self.host_revision(), bytes))
    }

    pub fn host_saved(
        &mut self,
        revision: u64,
        bytes: std::sync::Arc<Vec<u8>>,
    ) -> Result<bool, String> {
        let (index, id) = self.active_ids().ok_or("Open a PDF first.")?;
        let view = &self.views[index];
        let doc = self.session.get(id).ok_or("The PDF is unavailable.")?;
        let draft = view.pending_edit.is_some()
            || view.content.blocked().is_some()
            || view
                .forms
                .focus
                .as_ref()
                .is_some_and(|f| crate::forms_ui::draft_edit(f, &doc.form).is_some())
            || view
                .fill_text
                .as_ref()
                .is_some_and(|text| !text.text.trim().is_empty())
            || view
                .line_editor
                .as_ref()
                .is_some_and(|editor| editor.has_unsaved_text());
        if draft || doc.edit_generation() != revision {
            return Ok(false);
        }
        self.session
            .mark_saved(id, bytes, None)
            .map_err(|e| e.to_string())?;
        if let Some(doc) = self.session.get(id) {
            self.views[index].document_changed(&doc.info);
        }
        Ok(true)
    }
}
