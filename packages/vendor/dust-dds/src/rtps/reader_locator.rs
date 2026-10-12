use crate::transport::types::{CacheChange, Locator, SequenceNumber};

pub struct RtpsReaderLocator {
    locator: Locator,
    _expects_inline_qos: bool,
    highest_sent_change_sn: SequenceNumber,
}

impl RtpsReaderLocator {
    pub fn new(locator: Locator, expects_inline_qos: bool) -> Self {
        Self {
            locator,
            _expects_inline_qos: expects_inline_qos,
            highest_sent_change_sn: 0,
        }
    }

    pub fn locator(&self) -> Locator {
        self.locator
    }

    pub fn next_unsent_change<'a>(
        &'a mut self,
        writer_history_cache: impl Iterator<Item = &'a CacheChange>,
    ) -> Option<SequenceNumber> {
        // unsent_changes := { changes SUCH_THAT change.sequenceNumber > this.highestSentChangeSN }
        // IF unsent_changes == <empty> return SEQUENCE_NUMBER_INVALID
        // ELSE return MIN { unsent_changes.sequenceNumber }

        writer_history_cache
            .map(|cc| cc.sequence_number)
            .filter(|sn| sn > &self.highest_sent_change_sn)
            .min()
    }

    pub fn set_highest_sent_change_sn(&mut self, highest_sent_change_sn: SequenceNumber) {
        self.highest_sent_change_sn = highest_sent_change_sn;
    }
}
