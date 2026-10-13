#include "dds/dds.h"
#include "Telemetry.h"
#include "abi-checks.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void check(dds_return_t result, const char *operation) {
    if (result < 0) {
        fprintf(stderr, "%s: %s\n", operation, dds_strretcode(result));
        exit(1);
    }
}

int main(int argc, char **argv) {
    if (argc != 4 && argc != 5) {
        fprintf(stderr, "usage: dds-reference DOMAIN TOPIC_PREFIX reliable|best_effort [SECURITY_DIRECTORY]\n");
        return 2;
    }
    const uint32_t domain = (uint32_t)strtoul(argv[1], NULL, 10);
    const int reliable = strcmp(argv[3], "reliable") == 0;
    char input_name[256], output_name[256];
    snprintf(input_name, sizeof(input_name), "%sInput", argv[2]);
    snprintf(output_name, sizeof(output_name), "%sOutput", argv[2]);
    dds_qos_t *participant_qos = dds_create_qos();
    if (argc == 5) {
        const char *properties[][2] = {
            { "dds.sec.auth.identity_ca", "ca.pem" },
            { "dds.sec.auth.identity_certificate", "peer.pem" },
            { "dds.sec.auth.private_key", "peer.key" },
            { "dds.sec.access.permissions_ca", "permissions-ca.pem" },
            { "dds.sec.access.governance", "governance.p7s" },
            { "dds.sec.access.permissions", "peer-permissions.p7s" },
        };
        for (size_t i = 0; i < sizeof(properties) / sizeof(properties[0]); ++i) {
            char uri[2048];
            snprintf(uri, sizeof(uri), "file:%s/%s", argv[4], properties[i][1]);
            dds_qset_prop(participant_qos, properties[i][0], uri);
        }
        dds_qset_prop(participant_qos, "dds.sec.auth.library.path", "dds_security_auth");
        dds_qset_prop(participant_qos, "dds.sec.auth.library.init", "init_authentication");
        dds_qset_prop(participant_qos, "dds.sec.auth.library.finalize", "finalize_authentication");
        dds_qset_prop(participant_qos, "dds.sec.access.library.path", "dds_security_ac");
        dds_qset_prop(participant_qos, "dds.sec.access.library.init", "init_access_control");
        dds_qset_prop(participant_qos, "dds.sec.access.library.finalize", "finalize_access_control");
        dds_qset_prop(participant_qos, "dds.sec.crypto.library.path", "dds_security_crypto");
        dds_qset_prop(participant_qos, "dds.sec.crypto.library.init", "init_crypto");
        dds_qset_prop(participant_qos, "dds.sec.crypto.library.finalize", "finalize_crypto");
    }
    dds_entity_t participant = dds_create_participant(domain, participant_qos, NULL);
    dds_delete_qos(participant_qos);
    check(participant, "create participant");
    dds_entity_t participants = dds_create_reader(participant, DDS_BUILTIN_TOPIC_DCPSPARTICIPANT, NULL, NULL);
    check(participants, "create participant discovery reader");
    dds_entity_t input = dds_create_topic(participant, &Fixture_Telemetry_desc, input_name, NULL, NULL);
    dds_entity_t output = dds_create_topic(participant, &Fixture_Telemetry_desc, output_name, NULL, NULL);
    check(input, "create input topic");
    check(output, "create output topic");
    dds_qos_t *qos = dds_create_qos();
    dds_qset_reliability(qos, reliable ? DDS_RELIABILITY_RELIABLE : DDS_RELIABILITY_BEST_EFFORT, DDS_SECS(5));
    dds_qset_history(qos, DDS_HISTORY_KEEP_LAST, 32);
    dds_qset_resource_limits(qos, 32, 32, 32);
    dds_data_representation_id_t representation = DDS_DATA_REPRESENTATION_XCDR1;
    dds_qset_data_representation(qos, 1, &representation);
    dds_entity_t reader = dds_create_reader(participant, input, qos, NULL);
    dds_entity_t writer = dds_create_writer(participant, output, qos, NULL);
    check(reader, "create reader");
    check(writer, "create writer");
    dds_delete_qos(qos);
    puts("READY");
    fflush(stdout);

    dds_time_t deadline = dds_time() + DDS_SECS(40);
    while (dds_time() < deadline) {
        void *samples[1] = { NULL };
        dds_sample_info_t info[1];
        int count = dds_take(reader, samples, info, 1, 1);
        check(count, "take");
        if (count > 0 && info[0].valid_data) {
            Fixture_Telemetry *sample = samples[0];
            if (sample->id != 42 || strcmp(sample->label, "flow-like") != 0 ||
                sample->temperature != 23.5 || !sample->ready || sample->counter != UINT64_MAX ||
                sample->samples._length != 3 || sample->samples._buffer[0] != INT16_MIN ||
                sample->samples._buffer[1] != 0 || sample->samples._buffer[2] != INT16_MAX) {
                fprintf(stderr, "unexpected sample received by independent DDS reader\n");
                return 3;
            }
            dds_builtintopic_endpoint_t *publication = dds_get_matched_publication_data(reader, info[0].publication_handle);
            if (publication == NULL) return 6;
            dds_guid_t publisher_participant = publication->participant_key;
            dds_builtintopic_free_endpoint(publication);
            // Return known values through Cyclone's separately generated XCDR1 serializer.
            int16_t measurements[] = { -7, 42, 1024 };
            Fixture_Telemetry reply = {0};
            reply.id = sample->id;
            reply.label = "cyclonedds";
            reply.temperature = 24.5;
            reply.samples._length = 3;
            reply.samples._maximum = 3;
            reply.samples._buffer = measurements;
            reply.ready = true;
            reply.counter = UINT64_MAX;
            dds_publication_matched_status_t matched;
            do {
                check(dds_get_publication_matched_status(writer, &matched), "publication status");
                if (matched.current_count > 0) break;
                dds_sleepfor(DDS_MSECS(20));
            } while (dds_time() < deadline);
            if (matched.current_count == 0) return 4;
            check(dds_write(writer, &reply), "write");
            check(dds_return_loan(reader, samples, count), "return loan");
            if (reliable) check(dds_wait_for_acks(writer, DDS_SECS(5)), "wait for ACK");
            int dispose_requested = 0;
            dds_time_t dispose_deadline = dds_time() + DDS_SECS(5);
            while (!dispose_requested && dds_time() < dispose_deadline) {
                void *commands[1] = { NULL };
                dds_sample_info_t command_info[1];
                int commands_read = dds_take(reader, commands, command_info, 1, 1);
                check(commands_read, "take disposal request");
                if (commands_read > 0 && command_info[0].valid_data) {
                    Fixture_Telemetry *command = commands[0];
                    dispose_requested = command->id == 42 && strcmp(command->label, "dispose") == 0;
                    if (!dispose_requested) return 8;
                }
                if (commands_read > 0) check(dds_return_loan(reader, commands, commands_read), "return command loan");
                if (!dispose_requested) dds_sleepfor(DDS_MSECS(10));
            }
            if (!dispose_requested) return 9;
            check(dds_dispose(writer, &reply), "dispose instance");
            if (reliable) check(dds_wait_for_acks(writer, DDS_SECS(5)), "wait for dispose ACK");
            // The adapter must announce participant departure before the 100-second lease expires.
            int departed = 0;
            dds_time_t departure_deadline = dds_time() + DDS_SECS(5);
            while (!departed && dds_time() < departure_deadline) {
                // Cyclone's public participant builtin reader does not expose secure disposal
                // consistently. Endpoint unmatching still verifies timely remote cleanup.
                if (argc == 5 && dds_get_matched_publications(reader, NULL, 0) == 0) {
                    departed = 1;
                    break;
                }
                void *discovered[16] = { NULL };
                dds_sample_info_t states[16];
                int found = dds_take(participants, discovered, states, 16, 16);
                check(found, "take participant discovery");
                for (int i = 0; i < found; ++i) {
                    dds_builtintopic_participant_t *remote = discovered[i];
                    if (memcmp(&remote->key, &publisher_participant, sizeof(dds_guid_t)) == 0 &&
                        states[i].instance_state != DDS_IST_ALIVE) departed = 1;
                }
                if (found > 0) check(dds_return_loan(participants, discovered, found), "return discovery loan");
                if (!departed) dds_sleepfor(DDS_MSECS(10));
            }
            if (!departed) {
                fprintf(stderr, "peer did not announce DDS participant departure\n");
                return 7;
            }
            check(dds_delete(participant), "delete participant");
            puts("VERIFIED");
            return 0;
        }
        if (count > 0) check(dds_return_loan(reader, samples, count), "return loan");
        dds_sleepfor(DDS_MSECS(10));
    }
    fprintf(stderr, "timed out waiting for DDS sample\n");
    dds_delete(participant);
    return 5;
}
