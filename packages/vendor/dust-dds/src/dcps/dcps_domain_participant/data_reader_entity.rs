use crate::{
    builtin_topics::PublicationBuiltinTopicData,
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::DataReaderQos,
        qos_policy::{DestinationOrderQosPolicyKind, HistoryQosPolicyKind, OwnershipQosPolicyKind},
        sample_info::{InstanceStateKind, SampleInfo, SampleStateKind, ViewStateKind},
        status::SampleRejectedStatusKind,
        time::{Duration, DurationKind, TIME_INVALID_NSEC, TIME_INVALID_SEC, Time},
    },
    transport::types::{ChangeKind, Guid},
};
use alloc::{string::ToString, sync::Arc, vec::Vec};

pub type SampleList = Vec<(Arc<[u8]>, SampleInfo)>;

pub enum AddChangeResult {
    Added,
    NotAdded,
    Rejected(InstanceHandle, SampleRejectedStatusKind),
}

pub struct InstanceState {
    handle: InstanceHandle,
    last_accepted_source_timestamp: Option<Time>,
    view_state: ViewStateKind,
    instance_state: InstanceStateKind,
    most_recent_disposed_generation_count: i32,
    most_recent_no_writers_generation_count: i32,
    last_received_time_stamp: Time,
    registered_writers: Vec<[u8; 16]>,
}

impl InstanceState {
    pub fn new(handle: InstanceHandle) -> Self {
        Self {
            handle,
            last_accepted_source_timestamp: None,
            view_state: ViewStateKind::New,
            instance_state: InstanceStateKind::Alive,
            most_recent_disposed_generation_count: 0,
            most_recent_no_writers_generation_count: 0,
            last_received_time_stamp: Time::new(TIME_INVALID_SEC, TIME_INVALID_NSEC),
            registered_writers: Vec::new(),
        }
    }

    pub fn update_state(
        &mut self,
        change_kind: ChangeKind,
        writer_guid: [u8; 16],
        now: Option<Time>,
        source_timestamp: Option<Time>,
    ) {
        match change_kind {
            ChangeKind::Alive | ChangeKind::AliveFiltered => {
                if !self.registered_writers.contains(&writer_guid) {
                    self.registered_writers.push(writer_guid);
                }
                match self.instance_state {
                    InstanceStateKind::Alive => (),
                    InstanceStateKind::NotAliveDisposed => {
                        self.instance_state = InstanceStateKind::Alive;
                        self.most_recent_disposed_generation_count += 1;
                    }
                    InstanceStateKind::NotAliveNoWriters => {
                        self.instance_state = InstanceStateKind::Alive;
                        self.most_recent_no_writers_generation_count += 1;
                    }
                }
            }
            ChangeKind::NotAliveDisposed => {
                self.instance_state = InstanceStateKind::NotAliveDisposed;
            }
            ChangeKind::NotAliveUnregistered => {
                self.registered_writers.retain(|x| x != &writer_guid);
                if self.registered_writers.is_empty()
                    && self.instance_state == InstanceStateKind::Alive
                {
                    self.instance_state = InstanceStateKind::NotAliveNoWriters;
                }
            }
            ChangeKind::NotAliveDisposedUnregistered => {
                self.registered_writers.retain(|x| x != &writer_guid);
                if self.instance_state == InstanceStateKind::Alive {
                    if self.registered_writers.is_empty() {
                        self.instance_state = InstanceStateKind::NotAliveNoWriters;
                    } else {
                        self.instance_state = InstanceStateKind::NotAliveDisposed;
                    }
                }
            }
        }

        match self.view_state {
            ViewStateKind::New => (),
            ViewStateKind::NotNew => {
                if matches!(
                    change_kind,
                    ChangeKind::NotAliveDisposed
                        | ChangeKind::NotAliveUnregistered
                        | ChangeKind::NotAliveDisposedUnregistered
                ) {
                    self.view_state = ViewStateKind::New;
                }
            }
        }
        if let Some(t) = now {
            self.last_received_time_stamp = t;
        }
        if let Some(t) = source_timestamp {
            self.last_accepted_source_timestamp = Some(t);
        }
    }

    pub fn mark_viewed(&mut self) {
        self.view_state = ViewStateKind::NotNew;
    }

    pub fn remove_writer(&mut self, writer_guid: &[u8; 16]) {
        self.registered_writers.retain(|x| x != writer_guid);
        if self.registered_writers.is_empty() && self.instance_state == InstanceStateKind::Alive {
            self.instance_state = InstanceStateKind::NotAliveNoWriters;
            self.view_state = ViewStateKind::New;
        }
    }

    pub fn has_registered_writer(&self, writer_guid: &[u8; 16]) -> bool {
        self.registered_writers.contains(writer_guid)
    }

    pub fn handle(&self) -> &InstanceHandle {
        &self.handle
    }

    pub fn last_received_time_stamp(&self) -> Time {
        self.last_received_time_stamp
    }
}

#[derive(Debug)]
pub struct ReaderSample {
    pub kind: ChangeKind,
    pub writer_guid: [u8; 16],
    pub instance_handle: InstanceHandle,
    pub source_timestamp: Option<Time>,
    pub reception_timestamp: Time,
    pub data_value: Arc<[u8]>,
    pub sample_state: SampleStateKind,
    pub disposed_generation_count: i32,
    pub no_writers_generation_count: i32,
}

pub struct InstanceOwnership {
    pub instance_handle: InstanceHandle,
    pub owner_handle: [u8; 16],
    pub last_received_time: Time,
}

pub struct DataReaderEntity<T> {
    pub instance_handle: InstanceHandle,
    pub sample_list: Vec<ReaderSample>,
    pub qos: DataReaderQos,
    pub topic_name: Arc<str>,
    pub matched_publication_list: Vec<PublicationBuiltinTopicData>,
    pub enabled: bool,
    pub instances: Vec<InstanceState>,
    pub instance_ownership: Vec<InstanceOwnership>,
    pub transport_reader: T,
}

impl<T> DataReaderEntity<T> {
    pub fn new(
        instance_handle: InstanceHandle,
        qos: DataReaderQos,
        topic_name: Arc<str>,
        transport_reader: T,
    ) -> Self {
        Self {
            instance_handle,
            sample_list: Vec::new(),
            qos,
            topic_name,
            matched_publication_list: Vec::new(),
            enabled: false,
            instances: Vec::new(),
            instance_ownership: Vec::new(),
            transport_reader,
        }
    }

    pub fn remove_stale_samples(&mut self, now: Time) {
        let destination_order = self.qos.destination_order.kind;
        self.sample_list.retain(|sample| {
            let timestamp = match destination_order {
                DestinationOrderQosPolicyKind::BySourceTimestamp => sample.source_timestamp,
                DestinationOrderQosPolicyKind::ByReceptionTimestamp => {
                    Some(sample.reception_timestamp)
                }
            };
            if let Some(timestamp) = timestamp {
                if let Some(matched_publication) = self
                    .matched_publication_list
                    .iter()
                    .find(|x| x.key().value == sample.writer_guid)
                {
                    if let DurationKind::Finite(lifespan_duration) =
                        matched_publication.lifespan().duration
                    {
                        if now >= timestamp + lifespan_duration {
                            return false;
                        }
                    }
                }
            }
            true
        });
    }

    pub fn create_sample_collection(
        &mut self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
        take: bool,
    ) -> DdsResult<SampleList> {
        if self.sample_list.is_empty() || max_samples <= 0 {
            return Err(DdsError::NoData);
        }

        if let Some(h) = specific_instance_handle {
            if !self.instances.iter().any(|x| x.handle() == h) {
                return Err(DdsError::BadParameter);
            }
        };

        struct InstanceInCollection {
            handle: InstanceHandle,
            instance_index: usize,
            instance_state: InstanceState,
            most_recent_sample_absolute_generation_rank: i32,
            sample_rank_counter: i32,
        }

        let match_all_sample_states = sample_states.len() >= 2;
        let match_all_view_states = view_states.len() >= 2;
        let match_all_instance_states = instance_states.len() >= 3;

        let estimated_capacity = (max_samples as usize).min(self.sample_list.len());
        let mut samples = Vec::with_capacity(estimated_capacity);
        let mut instances_in_collection: Vec<InstanceInCollection> =
            Vec::with_capacity(self.instances.len().min(4));

        self.sample_list.retain_mut(|cache_change| {
            if samples.len() as i32 == max_samples {
                return true;
            }

            if let Some(h) = specific_instance_handle {
                if &cache_change.instance_handle != h {
                    return true;
                }
            };

            let (instance_idx, instance) = if self.instances.len() == 1
                && self.instances[0].handle == cache_change.instance_handle
            {
                (0, &self.instances[0])
            } else {
                match self
                    .instances
                    .iter()
                    .enumerate()
                    .find(|(_, x)| x.handle == cache_change.instance_handle)
                {
                    Some(entry) => entry,
                    None => return true,
                }
            };

            if (!match_all_sample_states && !sample_states.contains(&cache_change.sample_state))
                || (!match_all_view_states && !view_states.contains(&instance.view_state))
                || (!match_all_instance_states
                    && !instance_states.contains(&instance.instance_state))
            {
                return true;
            }

            let instance_in_coll = if instances_in_collection.len() == 1
                && instances_in_collection[0].handle == cache_change.instance_handle
            {
                &mut instances_in_collection[0]
            } else {
                match instances_in_collection
                    .iter_mut()
                    .find(|x| x.handle == cache_change.instance_handle)
                {
                    Some(entry) => entry,
                    None => {
                        instances_in_collection.push(InstanceInCollection {
                            handle: cache_change.instance_handle,
                            instance_index: instance_idx,
                            instance_state: InstanceState::new(cache_change.instance_handle),
                            most_recent_sample_absolute_generation_rank: 0,
                            sample_rank_counter: 0,
                        });
                        instances_in_collection.last_mut().unwrap()
                    }
                }
            };

            instance_in_coll.instance_state.update_state(
                cache_change.kind,
                cache_change.writer_guid,
                None,
                None,
            );
            let sample_state = cache_change.sample_state;
            let view_state = instance.view_state;
            let instance_state = instance.instance_state;

            let absolute_generation_rank = (instance.most_recent_disposed_generation_count
                + instance.most_recent_no_writers_generation_count)
                - (instance_in_coll
                    .instance_state
                    .most_recent_disposed_generation_count
                    + instance_in_coll
                        .instance_state
                        .most_recent_no_writers_generation_count);

            instance_in_coll.most_recent_sample_absolute_generation_rank = absolute_generation_rank;

            let (data, valid_data) = match cache_change.kind {
                ChangeKind::Alive | ChangeKind::AliveFiltered => {
                    (cache_change.data_value.clone(), true)
                }
                ChangeKind::NotAliveDisposed
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered => {
                    (cache_change.data_value.clone(), false)
                }
            };

            let sample_info = SampleInfo {
                sample_state,
                view_state,
                instance_state,
                disposed_generation_count: cache_change.disposed_generation_count,
                no_writers_generation_count: cache_change.no_writers_generation_count,
                sample_rank: 0,     // To be filled up after collection is created
                generation_rank: 0, // To be filled up after collection is created
                absolute_generation_rank,
                source_timestamp: cache_change.source_timestamp,
                instance_handle: cache_change.instance_handle,
                publication_handle: InstanceHandle::new(cache_change.writer_guid),
                valid_data,
            };

            samples.push((data, sample_info));

            if take {
                false
            } else {
                cache_change.sample_state = SampleStateKind::Read;
                true
            }
        });

        // After the collection is created, update relative generation rank and sample rank in a single reverse pass
        if instances_in_collection.len() == 1 {
            let inst = &mut instances_in_collection[0];
            let most_recent_rank = inst.most_recent_sample_absolute_generation_rank;
            for (i, (_, sample_info)) in samples.iter_mut().rev().enumerate() {
                sample_info.generation_rank =
                    sample_info.absolute_generation_rank - most_recent_rank;
                sample_info.sample_rank = i as i32;
            }
            self.instances[inst.instance_index].mark_viewed();
        } else {
            for (_, sample_info) in samples.iter_mut().rev() {
                let instance_in_coll = instances_in_collection
                    .iter_mut()
                    .find(|x| x.handle == sample_info.instance_handle)
                    .expect("Instance handle must exist on collection");

                sample_info.generation_rank = sample_info.absolute_generation_rank
                    - instance_in_coll.most_recent_sample_absolute_generation_rank;

                sample_info.sample_rank = instance_in_coll.sample_rank_counter;
                instance_in_coll.sample_rank_counter += 1;
            }

            for inst in &instances_in_collection {
                self.instances[inst.instance_index].mark_viewed();
            }
        }

        if samples.is_empty() {
            Err(DdsError::NoData)
        } else {
            Ok(samples)
        }
    }

    pub fn next_instance(
        &mut self,
        previous_handle: &Option<InstanceHandle>,
    ) -> Option<InstanceHandle> {
        match previous_handle {
            Some(p) => self
                .instances
                .iter()
                .map(|x| x.handle())
                .filter(|&h| h > p)
                .min()
                .cloned(),
            None => self.instances.iter().map(|x| x.handle()).min().cloned(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn add_reader_change(
        &mut self,
        writer_guid: Guid,
        data_value: Arc<[u8]>,
        change_kind: ChangeKind,
        change_instance_handle: [u8; 16],
        change_source_timestamp: Option<Time>,
        reception_timestamp: Time,
    ) -> DdsResult<AddChangeResult> {
        let instance_handle = InstanceHandle::new(change_instance_handle);

        if matches!(
            change_kind,
            ChangeKind::NotAliveDisposed
                | ChangeKind::NotAliveUnregistered
                | ChangeKind::NotAliveDisposedUnregistered
        ) && !self
            .instances
            .iter()
            .any(|x| x.handle() == &instance_handle)
        {
            return Err(DdsError::Error(
                "Received message changing state of unknown instance".to_string(),
            ));
        }

        // data_reader exclusive access if the writer is not allowed to write the sample do an early return
        if self.qos.ownership.kind == OwnershipQosPolicyKind::Exclusive {
            // Get the InstanceHandle of the data writer owning this instance
            if let Some(instance_owner) = self
                .instance_ownership
                .iter()
                .find(|x| x.instance_handle == instance_handle)
            {
                let instance_writer = InstanceHandle::new(writer_guid.into());
                let Some(sample_owner) = self
                    .matched_publication_list
                    .iter()
                    .find(|x| x.key().value == instance_owner.owner_handle.as_ref())
                else {
                    return Ok(AddChangeResult::NotAdded);
                };
                let Some(sample_writer) = self
                    .matched_publication_list
                    .iter()
                    .find(|x| &x.key().value == instance_writer.as_ref())
                else {
                    return Ok(AddChangeResult::NotAdded);
                };
                if instance_owner.owner_handle != writer_guid
                    && sample_writer.ownership_strength().value
                        <= sample_owner.ownership_strength().value
                {
                    return Ok(AddChangeResult::NotAdded);
                }
            }
        }

        let is_sample_of_interest_based_on_time = {
            if self.qos.time_based_filter.minimum_separation
                > DurationKind::Finite(Duration::new(0, 0))
            {
                if let Some(instance) = self.instances.iter().find(|x| x.handle == instance_handle)
                {
                    if let Some(last_accepted_time) = instance.last_accepted_source_timestamp {
                        if let Some(sample_source_time) = change_source_timestamp {
                            if sample_source_time >= last_accepted_time {
                                let sample_separation = sample_source_time - last_accepted_time;
                                DurationKind::Finite(sample_separation)
                                    >= self.qos.time_based_filter.minimum_separation
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            }
        };

        if !is_sample_of_interest_based_on_time {
            return Ok(AddChangeResult::NotAdded);
        }

        let is_max_samples_limit_reached = {
            let total_samples = self
                .sample_list
                .iter()
                .filter(|cc| cc.kind == ChangeKind::Alive)
                .count();

            total_samples == self.qos.resource_limits.max_samples
        };
        let is_max_instances_limit_reached = {
            let mut instance_handle_list = Vec::new();
            for sample_handle in self.sample_list.iter().map(|x| x.instance_handle) {
                if !instance_handle_list.contains(&sample_handle) {
                    instance_handle_list.push(sample_handle);
                }
            }

            if instance_handle_list.contains(&instance_handle) {
                false
            } else {
                instance_handle_list.len() == self.qos.resource_limits.max_instances
            }
        };
        let is_max_samples_per_instance_limit_reached = {
            let total_samples_of_instance = self
                .sample_list
                .iter()
                .filter(|cc| cc.instance_handle == instance_handle)
                .count();

            total_samples_of_instance == self.qos.resource_limits.max_samples_per_instance
        };
        if is_max_samples_limit_reached {
            return Ok(AddChangeResult::Rejected(
                instance_handle,
                SampleRejectedStatusKind::RejectedBySamplesLimit,
            ));
        } else if is_max_instances_limit_reached {
            return Ok(AddChangeResult::Rejected(
                instance_handle,
                SampleRejectedStatusKind::RejectedByInstancesLimit,
            ));
        } else if is_max_samples_per_instance_limit_reached {
            return Ok(AddChangeResult::Rejected(
                instance_handle,
                SampleRejectedStatusKind::RejectedBySamplesPerInstanceLimit,
            ));
        }
        let num_alive_samples_of_instance = self
            .sample_list
            .iter()
            .filter(|cc| cc.instance_handle == instance_handle && cc.kind == ChangeKind::Alive)
            .count() as u32;

        if let HistoryQosPolicyKind::KeepLast(depth) = self.qos.history.kind {
            if depth == num_alive_samples_of_instance {
                let index_sample_to_remove = self
                    .sample_list
                    .iter()
                    .position(|cc| {
                        cc.instance_handle == instance_handle && cc.kind == ChangeKind::Alive
                    })
                    .expect("Samples must exist");
                self.sample_list.remove(index_sample_to_remove);
            }
        }

        let (disposed_generation_count, no_writers_generation_count) = match change_kind {
            ChangeKind::Alive | ChangeKind::AliveFiltered => {
                match self
                    .instances
                    .iter_mut()
                    .find(|x| x.handle() == &instance_handle)
                {
                    Some(x) => {
                        x.update_state(
                            change_kind,
                            writer_guid.into(),
                            Some(reception_timestamp),
                            change_source_timestamp,
                        );
                        (
                            x.most_recent_disposed_generation_count,
                            x.most_recent_no_writers_generation_count,
                        )
                    }
                    None => {
                        let mut s = InstanceState::new(instance_handle);
                        s.update_state(
                            change_kind,
                            writer_guid.into(),
                            Some(reception_timestamp),
                            change_source_timestamp,
                        );
                        let counts = (
                            s.most_recent_disposed_generation_count,
                            s.most_recent_no_writers_generation_count,
                        );
                        self.instances.push(s);
                        counts
                    }
                }
            }
            ChangeKind::NotAliveDisposed
            | ChangeKind::NotAliveUnregistered
            | ChangeKind::NotAliveDisposedUnregistered => {
                let instance = self
                    .instances
                    .iter_mut()
                    .find(|x| x.handle() == &instance_handle)
                    .expect("Instance must exist");
                instance.update_state(
                    change_kind,
                    writer_guid.into(),
                    Some(reception_timestamp),
                    change_source_timestamp,
                );
                (
                    instance.most_recent_disposed_generation_count,
                    instance.most_recent_no_writers_generation_count,
                )
            }
        };

        let sample = ReaderSample {
            kind: change_kind,
            writer_guid: writer_guid.into(),
            instance_handle,
            source_timestamp: change_source_timestamp,
            reception_timestamp,
            data_value,
            sample_state: SampleStateKind::NotRead,
            disposed_generation_count,
            no_writers_generation_count,
        };
        tracing::debug!(cache_change = ?sample, "Adding change to data reader history cache");

        match self.qos.destination_order.kind {
            DestinationOrderQosPolicyKind::BySourceTimestamp => {
                // Insert the element at the place where the first source timestamp is bigger than the currently received one
                let insert_position = self
                    .sample_list
                    .iter()
                    .position(|x| x.source_timestamp > sample.source_timestamp)
                    .unwrap_or(0);
                self.sample_list.insert(insert_position, sample);
            }
            DestinationOrderQosPolicyKind::ByReceptionTimestamp => self.sample_list.push(sample),
        }

        if self.qos.ownership.kind == OwnershipQosPolicyKind::Exclusive {
            if matches!(
                change_kind,
                ChangeKind::NotAliveDisposed
                    | ChangeKind::NotAliveUnregistered
                    | ChangeKind::NotAliveDisposedUnregistered
            ) {
                if let Some(i) = self
                    .instance_ownership
                    .iter()
                    .position(|x| x.instance_handle == instance_handle)
                {
                    self.instance_ownership.remove(i);
                }
            } else {
                match self
                    .instance_ownership
                    .iter_mut()
                    .find(|x| x.instance_handle == instance_handle)
                {
                    Some(x) => {
                        x.owner_handle = writer_guid.into();
                        if x.last_received_time < reception_timestamp {
                            x.last_received_time = reception_timestamp;
                        }
                    }
                    None => self.instance_ownership.push(InstanceOwnership {
                        instance_handle,
                        last_received_time: reception_timestamp,
                        owner_handle: writer_guid.into(),
                    }),
                }
            }
        }
        Ok(AddChangeResult::Added)
    }

    pub fn get_matched_publications(&self) -> Vec<InstanceHandle> {
        self.matched_publication_list
            .iter()
            .map(|x| InstanceHandle::new(x.key().value))
            .collect()
    }

    pub fn read(
        &mut self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<SampleList> {
        if !self.enabled {
            return Err(DdsError::NotEnabled);
        }

        self.create_sample_collection(
            max_samples,
            sample_states,
            view_states,
            instance_states,
            specific_instance_handle,
            false,
        )
    }

    pub fn take(
        &mut self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<SampleList> {
        if !self.enabled {
            return Err(DdsError::NotEnabled);
        }

        self.create_sample_collection(
            max_samples,
            sample_states,
            view_states,
            instance_states,
            specific_instance_handle,
            true,
        )
    }
}
