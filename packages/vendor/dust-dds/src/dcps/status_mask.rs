use crate::infrastructure::status::StatusKind;

/// A mask representation of [`StatusKind`] variants stored as bits in a `u16`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusMask(u16);

impl StatusMask {
    /// Checks if the status kind is enabled in this mask.
    pub fn is_enabled(&self, status: &StatusKind) -> bool {
        (self.0 & Self::status_kind_bit(status)) != 0
    }

    const fn status_kind_bit(status: &StatusKind) -> u16 {
        match status {
            StatusKind::InconsistentTopic => 1 << 0,
            StatusKind::OfferedDeadlineMissed => 1 << 1,
            StatusKind::RequestedDeadlineMissed => 1 << 2,
            StatusKind::OfferedIncompatibleQos => 1 << 3,
            StatusKind::RequestedIncompatibleQos => 1 << 4,
            StatusKind::SampleLost => 1 << 5,
            StatusKind::SampleRejected => 1 << 6,
            StatusKind::DataOnReaders => 1 << 7,
            StatusKind::DataAvailable => 1 << 8,
            StatusKind::LivelinessLost => 1 << 9,
            StatusKind::LivelinessChanged => 1 << 10,
            StatusKind::PublicationMatched => 1 << 11,
            StatusKind::SubscriptionMatched => 1 << 12,
        }
    }
}

impl<'a> core::iter::FromIterator<&'a StatusKind> for StatusMask {
    fn from_iter<T: IntoIterator<Item = &'a StatusKind>>(iter: T) -> Self {
        let mut mask = 0;
        for status in iter {
            mask |= Self::status_kind_bit(status);
        }
        Self(mask)
    }
}

/// An iterator that yields the [`StatusKind`] variants enabled in a [`StatusMask`].
#[derive(Clone, Debug)]
pub struct StatusMaskIter {
    mask: u16,
    index: u8,
}

impl Iterator for StatusMaskIter {
    type Item = StatusKind;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < 13 {
            let bit = 1 << self.index;
            let current_index = self.index;
            self.index += 1;
            if (self.mask & bit) != 0 {
                let status = match current_index {
                    0 => StatusKind::InconsistentTopic,
                    1 => StatusKind::OfferedDeadlineMissed,
                    2 => StatusKind::RequestedDeadlineMissed,
                    3 => StatusKind::OfferedIncompatibleQos,
                    4 => StatusKind::RequestedIncompatibleQos,
                    5 => StatusKind::SampleLost,
                    6 => StatusKind::SampleRejected,
                    7 => StatusKind::DataOnReaders,
                    8 => StatusKind::DataAvailable,
                    9 => StatusKind::LivelinessLost,
                    10 => StatusKind::LivelinessChanged,
                    11 => StatusKind::PublicationMatched,
                    12 => StatusKind::SubscriptionMatched,
                    _ => unreachable!(),
                };
                return Some(status);
            }
        }
        None
    }
}

impl IntoIterator for StatusMask {
    type Item = StatusKind;
    type IntoIter = StatusMaskIter;

    fn into_iter(self) -> Self::IntoIter {
        StatusMaskIter {
            mask: self.0,
            index: 0,
        }
    }
}

impl IntoIterator for &StatusMask {
    type Item = StatusKind;
    type IntoIter = StatusMaskIter;

    fn into_iter(self) -> Self::IntoIter {
        StatusMaskIter {
            mask: self.0,
            index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_iterator() {
        let mask: StatusMask = [StatusKind::SampleLost, StatusKind::PublicationMatched]
            .iter()
            .collect();

        assert!(mask.is_enabled(&StatusKind::SampleLost));
        assert!(mask.is_enabled(&StatusKind::PublicationMatched));
        assert!(!mask.is_enabled(&StatusKind::DataAvailable));
    }

    #[test]
    fn test_into_iterator() {
        let mask: StatusMask = [StatusKind::SampleLost, StatusKind::PublicationMatched]
            .iter()
            .collect();

        let mut iter = mask.into_iter();
        assert_eq!(iter.next(), Some(StatusKind::SampleLost));
        assert_eq!(iter.next(), Some(StatusKind::PublicationMatched));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_into_iterator_for_reference() {
        let mask: StatusMask = [StatusKind::SampleLost, StatusKind::PublicationMatched]
            .iter()
            .collect();

        let mut iter = (&mask).into_iter();
        assert_eq!(iter.next(), Some(StatusKind::SampleLost));
        assert_eq!(iter.next(), Some(StatusKind::PublicationMatched));
        assert_eq!(iter.next(), None);
    }
}
