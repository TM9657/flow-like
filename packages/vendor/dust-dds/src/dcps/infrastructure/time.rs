use crate::xtypes::{
    dynamic_type::DynamicData,
    type_support::{Type, TypeSupport},
};
use core::ops::{Add, AddAssign, Sub};

/// Enumeration representing whether a duration is finite or infinite
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum DurationKind {
    /// Finite duration with the corresponding associated value
    Finite(Duration),
    /// Infinite duration
    Infinite,
}

impl Type for DurationKind {
    const TYPE: crate::xtypes::dynamic_type::DynamicType<'static> = Duration::TYPE;
}
impl TypeSupport for DurationKind {
    fn create_sample(src: &mut crate::xtypes::dynamic_type::DynamicData<'static>) -> Option<Self> {
        let duration = Duration::create_sample(src)?;
        match duration {
            DURATION_INFINITE => Some(DurationKind::Infinite),
            d => Some(DurationKind::Finite(d)),
        }
    }

    fn create_dynamic_sample(self) -> DynamicData<'static> {
        let value = match self {
            DurationKind::Finite(duration) => duration,
            DurationKind::Infinite => DURATION_INFINITE,
        };
        value.create_dynamic_sample()
    }
}

const DURATION_INFINITE_SEC: i32 = 0x7fffffff;
const DURATION_INFINITE_NSEC: u32 = 0xffffffff;
const DURATION_INFINITE: Duration = Duration {
    sec: DURATION_INFINITE_SEC,
    nanosec: DURATION_INFINITE_NSEC,
};

impl PartialOrd<DurationKind> for DurationKind {
    fn partial_cmp(&self, other: &DurationKind) -> Option<core::cmp::Ordering> {
        match self {
            DurationKind::Finite(this) => match other {
                DurationKind::Finite(v) => Duration::partial_cmp(this, v),
                DurationKind::Infinite => Some(core::cmp::Ordering::Less),
            },
            DurationKind::Infinite => match other {
                DurationKind::Finite(_) => Some(core::cmp::Ordering::Greater),
                DurationKind::Infinite => Some(core::cmp::Ordering::Equal),
            },
        }
    }
}

/// Structure representing a time interval with a nanosecond resolution.
#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy)]
pub struct Duration {
    sec: i32,
    nanosec: u32,
}

impl dust_dds::xtypes::type_support::TypeSupport for Duration {
    fn create_sample(
        src: &mut dust_dds::xtypes::dynamic_type::DynamicData,
    ) -> ::core::option::Option<Self> {
        Some(Self {
            sec: dust_dds::xtypes::data_storage::DataStorageMapping::try_from_storage(
                src.remove_value(0).ok()?,
            )
            .ok()?,
            nanosec: fraction_to_nanosec(
                dust_dds::xtypes::data_storage::DataStorageMapping::try_from_storage(
                    src.remove_value(1).ok()?,
                )
                .ok()?,
            ),
        })
    }
    fn create_dynamic_sample(self) -> dust_dds::xtypes::dynamic_type::DynamicData<'static> {
        let mut data =
            dust_dds::xtypes::dynamic_type::DynamicDataFactory::create_data(Self::get_type());
        data.set_value(
            0,
            dust_dds::xtypes::data_storage::DataStorageMapping::into_storage(self.sec),
        );
        data.set_value(
            1,
            dust_dds::xtypes::data_storage::DataStorageMapping::into_storage(nanosec_to_fraction(
                self.nanosec,
            )),
        );
        data
    }
}

impl dust_dds::xtypes::type_support::Type for Duration {
    const TYPE: dust_dds::xtypes::dynamic_type::DynamicType<'static> =
        dust_dds::xtypes::dynamic_type::DynamicType {
            descriptor: &dust_dds::xtypes::dynamic_type::TypeDescriptor {
                kind: dust_dds::xtypes::dynamic_type::TypeKind::STRUCTURE,
                name: "Duration",
                base_type: None,
                discriminator_type: None,
                bound: &[],
                element_type: None,
                key_element_type: None,
                extensibility_kind: dust_dds::xtypes::dynamic_type::ExtensibilityKind::Final,
                is_nested: true,
                is_autoid_hash: false,
            },
            member_list: &[
                dust_dds::xtypes::dynamic_type::DynamicTypeMember {
                    descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                        name: "sec",
                        id: 0,
                        r#type: <i32 as dust_dds::xtypes::type_support::Type>::TYPE,
                        default_value: None,
                        index: 0u32,
                        try_construct_kind:
                            dust_dds::xtypes::dynamic_type::TryConstructKind::Discard,
                        label: &[],
                        is_key: false,
                        is_optional: false,
                        is_must_understand: false,
                        is_shared: false,
                        is_default_label: false,
                        is_external: false,
                    },
                },
                dust_dds::xtypes::dynamic_type::DynamicTypeMember {
                    descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                        name: "fraction",
                        id: 1,
                        r#type: <u32 as dust_dds::xtypes::type_support::Type>::TYPE,
                        default_value: None,
                        index: 1u32,
                        try_construct_kind:
                            dust_dds::xtypes::dynamic_type::TryConstructKind::Discard,
                        label: &[],
                        is_key: false,
                        is_optional: false,
                        is_must_understand: false,
                        is_shared: false,
                        is_default_label: false,
                        is_external: false,
                    },
                },
            ],
        };
}

impl Duration {
    /// Construct a new [`Duration`] with the corresponding seconds and nanoseconds.
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        let sec = sec.saturating_add((nanosec / 1_000_000_000) as i32);
        let nanosec = nanosec % 1_000_000_000;
        Self { sec, nanosec }
    }

    /// Get a reference to the duration's sec.
    pub fn sec(&self) -> i32 {
        self.sec
    }

    /// Get a reference to the duration's nanosec.
    pub fn nanosec(&self) -> u32 {
        self.nanosec
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        let mut sec = self.sec.saturating_add(rhs.sec);
        let mut nanosec = (self.nanosec as u64) + (rhs.nanosec as u64);
        let sec_in_nanosec = nanosec / 1_000_000_000;
        nanosec -= sec_in_nanosec * 1_000_000_000;
        sec = sec.saturating_add(sec_in_nanosec as i32);
        Self {
            sec,
            nanosec: nanosec as u32,
        }
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        let mut sec = self.sec.saturating_sub(rhs.sec);
        let nanosec_diff = (self.nanosec as i64) - (rhs.nanosec as i64);
        let nanosec = if nanosec_diff < 0 {
            sec = sec.saturating_sub(1);
            (1_000_000_000 + nanosec_diff) as u32
        } else {
            self.nanosec - rhs.nanosec
        };
        Self { sec, nanosec }
    }
}

const fn fraction_to_nanosec(fraction: u32) -> u32 {
    ((fraction as u64 * 1_000_000_000) / (1u64 << 32)) as u32
}

const fn nanosec_to_fraction(nanosec: u32) -> u32 {
    (((nanosec as u64 * (1u64 << 32)) + (500_000_000)) / 1_000_000_000) as u32
}

impl From<crate::rtps::behavior_types::Duration> for Duration {
    fn from(value: crate::rtps::behavior_types::Duration) -> Self {
        Self {
            sec: value.seconds(),
            nanosec: fraction_to_nanosec(value.fraction()),
        }
    }
}

impl From<Duration> for crate::rtps::behavior_types::Duration {
    fn from(value: Duration) -> Self {
        crate::rtps::behavior_types::Duration::new(value.sec, nanosec_to_fraction(value.nanosec))
    }
}

impl From<crate::rtps_messages::types::Time> for Duration {
    fn from(value: crate::rtps_messages::types::Time) -> Self {
        Self {
            sec: value.seconds() as i32,
            nanosec: fraction_to_nanosec(value.fraction()),
        }
    }
}

impl From<Duration> for crate::rtps_messages::types::Time {
    fn from(value: Duration) -> Self {
        crate::rtps_messages::types::Time::new(value.sec as u32, nanosec_to_fraction(value.nanosec))
    }
}

impl From<core::time::Duration> for Duration {
    fn from(x: core::time::Duration) -> Self {
        Self::new(x.as_secs() as i32, x.subsec_nanos())
    }
}

impl From<Duration> for core::time::Duration {
    fn from(x: Duration) -> Self {
        core::time::Duration::new(x.sec.max(0) as u64, x.nanosec)
    }
}

/// Structure representing a time in Unix Epoch with a nanosecond resolution.
#[derive(Clone, PartialEq, Debug, Copy, PartialOrd, Eq, Ord)]
pub struct Time {
    sec: i32,
    nanosec: u32,
}

impl Time {
    /// Create a new [`Time`] with a number of seconds and nanoseconds
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        let sec = sec.saturating_add((nanosec / 1_000_000_000) as i32);
        let nanosec = nanosec % 1_000_000_000;
        Self { sec, nanosec }
    }

    /// Get the number of seconds contained by this time
    pub const fn sec(&self) -> i32 {
        self.sec
    }

    /// Get the number of nanoseconds contained by this time
    pub const fn nanosec(&self) -> u32 {
        self.nanosec
    }
}

impl From<crate::transport::types::Time> for Time {
    fn from(value: crate::transport::types::Time) -> Self {
        Self::new(value.sec(), value.nanosec())
    }
}

impl From<Time> for crate::transport::types::Time {
    fn from(value: Time) -> Self {
        Self::new(value.sec(), value.nanosec())
    }
}

impl Sub<Time> for Time {
    type Output = Duration;

    fn sub(self, rhs: Time) -> Self::Output {
        let lhs = Duration::new(self.sec, self.nanosec);
        let rhs = Duration::new(rhs.sec, rhs.nanosec);
        lhs - rhs
    }
}

impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        let mut sec = self.sec.saturating_add(rhs.sec);
        let mut nanosec = (self.nanosec as u64) + (rhs.nanosec as u64);
        let sec_in_nanosec = nanosec / 1_000_000_000;
        nanosec -= sec_in_nanosec * 1_000_000_000;
        sec = sec.saturating_add(sec_in_nanosec as i32);
        Self {
            sec,
            nanosec: nanosec as u32,
        }
    }
}
impl AddAssign<Duration> for Time {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

/// Pre-defined value representing a zero duration seconds
pub const DURATION_ZERO_SEC: i32 = 0;
/// Pre-defined value representing a zero duration nano seconds
pub const DURATION_ZERO_NSEC: u32 = 0;

/// Pre-defined value representing an invalid time
pub const TIME_INVALID_SEC: i32 = -1;
/// v value representing an invalid time nano seconds
pub const TIME_INVALID_NSEC: u32 = 0xffffffff;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dds_duration_to_rtps_duration() {
        let dds_duration = Duration::new(13, 500_000_000);

        let rtps_duration = crate::rtps::behavior_types::Duration::from(dds_duration);

        assert_eq!(rtps_duration.seconds(), 13);
        assert_eq!(rtps_duration.fraction(), 2u32.pow(31));
    }

    #[test]
    fn rtps_duration_to_dds_duration() {
        let rtps_duration = crate::rtps::behavior_types::Duration::new(13, 2u32.pow(31));

        let dds_duration = Duration::from(rtps_duration);

        assert_eq!(dds_duration.sec, 13);
        assert_eq!(dds_duration.nanosec, 500_000_000);
    }

    #[test]
    fn dds_duration_to_rtps_duration_to_dds_duration() {
        let dds_time = Duration::new(13, 200);

        let rtps_time = crate::rtps::behavior_types::Duration::from(dds_time);
        let dds_time_from_rtps_time = Duration::from(rtps_time);

        assert_eq!(dds_time, dds_time_from_rtps_time)
    }

    #[test]
    fn duration_ord() {
        let d1 = Duration::new(1, 100);
        let d2 = Duration::new(1, 200);
        let d3 = Duration::new(2, 50);

        assert!(d1 == d1);
        assert!(d1 < d2);
        assert!(d2 < d3);
        assert!(d1 < d3);
        assert_eq!(d1.cmp(&d1), std::cmp::Ordering::Equal);
        assert_eq!(d1.cmp(&d2), std::cmp::Ordering::Less);
        assert_eq!(d2.cmp(&d1), std::cmp::Ordering::Greater);
    }

    #[test]
    fn time_add_duration() {
        let t = Time::new(10, 500_000_000);
        let d = Duration::new(5, 600_000_000);
        let expected = Time::new(16, 100_000_000);
        assert_eq!(t + d, expected);
    }
}
