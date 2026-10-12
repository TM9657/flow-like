#include <open62541/server.h>
#include <open62541/server_config_default.h>
#include <open62541/server_pubsub.h>
#include <open62541/plugin/securitypolicy_default.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The standard open62541 subscriber writes server variables. A separate publisher
 * samples those variables, so the echo traverses both independent UADP codecs. */
static volatile UA_Boolean running = true;
static void stop(int sig) { (void)sig; running = false; }
static void check(UA_StatusCode status) {
    if(status != UA_STATUSCODE_GOOD) {
        fprintf(stderr, "open62541 fixture: %s\n", UA_StatusCode_name(status));
        exit(1);
    }
}
static const size_t types[] = {
    UA_TYPES_BOOLEAN, UA_TYPES_SBYTE, UA_TYPES_BYTE, UA_TYPES_INT16,
    UA_TYPES_UINT16, UA_TYPES_INT32, UA_TYPES_UINT32, UA_TYPES_INT64,
    UA_TYPES_UINT64, UA_TYPES_FLOAT, UA_TYPES_DOUBLE, UA_TYPES_STRING,
    UA_TYPES_DATETIME, UA_TYPES_BYTESTRING, UA_TYPES_STATUSCODE
};
#define FIELD_COUNT (sizeof(types) / sizeof(types[0]))

static UA_NodeId connection(UA_Server *server, char *url, UA_UInt16 publisher) {
    UA_PubSubConnectionConfig config;
    memset(&config, 0, sizeof(config));
    config.name = UA_STRING("Fixture UDP");
    config.enabled = true;
    config.transportProfileUri = UA_STRING("http://opcfoundation.org/UA-Profile/Transport/pubsub-udp-uadp");
    UA_NetworkAddressUrlDataType address = {UA_STRING_NULL, UA_STRING(url)};
    UA_Variant_setScalar(&config.address, &address, &UA_TYPES[UA_TYPES_NETWORKADDRESSURLDATATYPE]);
    config.publisherIdType = UA_PUBLISHERIDTYPE_UINT16;
    config.publisherId.uint16 = publisher;
    UA_NodeId id;
    check(UA_Server_addPubSubConnection(server, &config, &id));
    return id;
}

int main(int argc, char **argv) {
    if(argc != 3 && argc != 4) {
        fprintf(stderr, "usage: opcua-pubsub-peer input-opc.udp-url output-opc.udp-url [sign|encrypt]\n");
        return 2;
    }
    UA_MessageSecurityMode security_mode = UA_MESSAGESECURITYMODE_NONE;
    if(argc == 4) {
        if(strcmp(argv[3], "sign") == 0) security_mode = UA_MESSAGESECURITYMODE_SIGN;
        else if(strcmp(argv[3], "encrypt") == 0) security_mode = UA_MESSAGESECURITYMODE_SIGNANDENCRYPT;
        else return 2;
    }
    signal(SIGINT, stop);
    signal(SIGTERM, stop);
    UA_Server *server = UA_Server_new();
    check(UA_ServerConfig_setMinimal(UA_Server_getConfig(server), 0, NULL));
    UA_ServerConfig *server_config = UA_Server_getConfig(server);
    if(security_mode != UA_MESSAGESECURITYMODE_NONE) {
        server_config->pubSubConfig.securityPolicies = UA_malloc(sizeof(UA_PubSubSecurityPolicy));
        server_config->pubSubConfig.securityPoliciesSize = 1;
        check(UA_PubSubSecurityPolicy_Aes256Ctr(server_config->pubSubConfig.securityPolicies,
                                              server_config->logging));
    }
    UA_NodeId reader_connection = connection(server, argv[1], 7777);
    UA_NodeId writer_connection = connection(server, argv[2], 2235);

    UA_FieldMetaData metadata[FIELD_COUNT];
    UA_FieldTargetVariable targets[FIELD_COUNT];
    memset(metadata, 0, sizeof(metadata));
    memset(targets, 0, sizeof(targets));
    for(size_t i = 0; i < FIELD_COUNT; i++) {
        char name[32];
        snprintf(name, sizeof(name), "field_%zu", i);
        const UA_DataType *type = &UA_TYPES[types[i]];
        metadata[i].name = UA_STRING_ALLOC(name);
        metadata[i].dataType = type->typeId;
        metadata[i].builtInType = (UA_Byte)type->typeId.identifier.numeric;
        metadata[i].valueRank = -1;
        UA_VariableAttributes attr = UA_VariableAttributes_default;
        attr.displayName = UA_LOCALIZEDTEXT("en", name);
        attr.dataType = type->typeId;
        attr.valueRank = -1;
        attr.accessLevel = UA_ACCESSLEVELMASK_READ | UA_ACCESSLEVELMASK_WRITE;
        void *initial = UA_new(type);
        UA_Variant_setScalar(&attr.value, initial, type);
        UA_NodeId node = UA_NODEID_NUMERIC(1, (UA_UInt32)i + 50000);
        check(UA_Server_addVariableNode(server, node,
            UA_NODEID_NUMERIC(0, UA_NS0ID_OBJECTSFOLDER),
            UA_NODEID_NUMERIC(0, UA_NS0ID_ORGANIZES), UA_QUALIFIEDNAME(1, name),
            UA_NODEID_NUMERIC(0, UA_NS0ID_BASEDATAVARIABLETYPE), attr, NULL, NULL));
        UA_delete(initial, type);
        targets[i].targetVariable.targetNodeId = node;
        targets[i].targetVariable.attributeId = UA_ATTRIBUTEID_VALUE;
    }
    UA_ReaderGroupConfig reader_group;
    memset(&reader_group, 0, sizeof(reader_group));
    reader_group.name = UA_STRING("Fixture reader group");
    reader_group.securityMode = security_mode;
    if(security_mode != UA_MESSAGESECURITYMODE_NONE)
        reader_group.securityPolicy = &server_config->pubSubConfig.securityPolicies[0];
    UA_NodeId reader_group_id;
    check(UA_Server_addReaderGroup(server, reader_connection, &reader_group, &reader_group_id));
    UA_DataSetReaderConfig reader;
    memset(&reader, 0, sizeof(reader));
    reader.name = UA_STRING("Fixture reader");
    UA_UInt16 publisher = 2234;
    UA_Variant_setScalar(&reader.publisherId, &publisher, &UA_TYPES[UA_TYPES_UINT16]);
    reader.writerGroupId = 100;
    reader.dataSetWriterId = 62541;
    reader.dataSetMetaData.name = UA_STRING("Fixture scalar fields");
    reader.dataSetMetaData.fields = metadata;
    reader.dataSetMetaData.fieldsSize = FIELD_COUNT;
    UA_NodeId reader_id;
    check(UA_Server_addDataSetReader(server, reader_group_id, &reader, &reader_id));
    check(UA_Server_DataSetReader_createTargetVariables(server, reader_id, FIELD_COUNT, targets));

    UA_PublishedDataSetConfig data;
    memset(&data, 0, sizeof(data));
    data.name = UA_STRING("Fixture republished variables");
    data.publishedDataSetType = UA_PUBSUB_DATASET_PUBLISHEDITEMS;
    UA_NodeId data_id;
    check(UA_Server_addPublishedDataSet(server, &data, &data_id).addResult);
    for(size_t i = 0; i < FIELD_COUNT; i++) {
        UA_DataSetFieldConfig field;
        memset(&field, 0, sizeof(field));
        field.dataSetFieldType = UA_PUBSUB_DATASETFIELD_VARIABLE;
        field.field.variable.fieldNameAlias = metadata[i].name;
        field.field.variable.publishParameters.publishedVariable = targets[i].targetVariable.targetNodeId;
        field.field.variable.publishParameters.attributeId = UA_ATTRIBUTEID_VALUE;
        check(UA_Server_addDataSetField(server, data_id, &field, NULL).result);
    }
    UA_UadpWriterGroupMessageDataType group_message;
    UA_UadpWriterGroupMessageDataType_init(&group_message);
    group_message.networkMessageContentMask = UA_UADPNETWORKMESSAGECONTENTMASK_PUBLISHERID |
        UA_UADPNETWORKMESSAGECONTENTMASK_GROUPHEADER | UA_UADPNETWORKMESSAGECONTENTMASK_WRITERGROUPID |
        UA_UADPNETWORKMESSAGECONTENTMASK_PAYLOADHEADER | UA_UADPNETWORKMESSAGECONTENTMASK_SEQUENCENUMBER;
    UA_WriterGroupConfig writer_group;
    memset(&writer_group, 0, sizeof(writer_group));
    writer_group.name = UA_STRING("Fixture writer group");
    writer_group.publishingInterval = 25;
    writer_group.writerGroupId = 101;
    writer_group.encodingMimeType = UA_PUBSUB_ENCODING_UADP;
    writer_group.messageSettings.encoding = UA_EXTENSIONOBJECT_DECODED;
    writer_group.messageSettings.content.decoded.type = &UA_TYPES[UA_TYPES_UADPWRITERGROUPMESSAGEDATATYPE];
    writer_group.messageSettings.content.decoded.data = &group_message;
    writer_group.securityMode = security_mode;
    if(security_mode != UA_MESSAGESECURITYMODE_NONE)
        writer_group.securityPolicy = &server_config->pubSubConfig.securityPolicies[0];
    UA_NodeId writer_group_id;
    check(UA_Server_addWriterGroup(server, writer_connection, &writer_group, &writer_group_id));
    UA_UadpDataSetWriterMessageDataType data_message;
    UA_UadpDataSetWriterMessageDataType_init(&data_message);
    data_message.dataSetMessageContentMask = UA_UADPDATASETMESSAGECONTENTMASK_SEQUENCENUMBER |
        UA_UADPDATASETMESSAGECONTENTMASK_MAJORVERSION | UA_UADPDATASETMESSAGECONTENTMASK_MINORVERSION |
        UA_UADPDATASETMESSAGECONTENTMASK_TIMESTAMP;
    UA_DataSetWriterConfig writer;
    memset(&writer, 0, sizeof(writer));
    writer.name = UA_STRING("Fixture writer");
    writer.dataSetWriterId = 62542;
    /* open62541 1.4 counts allowed delta frames here; zero forces key frames. */
    writer.keyFrameCount = 0;
    writer.messageSettings.encoding = UA_EXTENSIONOBJECT_DECODED;
    writer.messageSettings.content.decoded.type = &UA_TYPES[UA_TYPES_UADPDATASETWRITERMESSAGEDATATYPE];
    writer.messageSettings.content.decoded.data = &data_message;
    check(UA_Server_addDataSetWriter(server, writer_group_id, data_id, &writer, NULL));
    if(security_mode != UA_MESSAGESECURITYMODE_NONE) {
        /* Public test keys only. Each direction has independent group key material. */
        UA_Byte signing[32], encrypting[32], nonce[4] = {1, 2, 3, 4};
        memset(signing, 11, sizeof(signing));
        memset(encrypting, 22, sizeof(encrypting));
        UA_ByteString sk = {sizeof(signing), signing};
        UA_ByteString ek = {sizeof(encrypting), encrypting};
        UA_ByteString kn = {sizeof(nonce), nonce};
        check(UA_Server_setReaderGroupEncryptionKeys(server, reader_group_id, 42, sk, ek, kn));
        memset(signing, 33, sizeof(signing));
        memset(encrypting, 44, sizeof(encrypting));
        for(size_t i = 0; i < sizeof(nonce); i++) nonce[i] += 4;
        check(UA_Server_setWriterGroupEncryptionKeys(server, writer_group_id, 43, sk, ek, kn));
    }
    check(UA_Server_setReaderGroupOperational(server, reader_group_id));
    check(UA_Server_setWriterGroupOperational(server, writer_group_id));
    for(size_t i = 0; i < FIELD_COUNT; i++) UA_String_clear(&metadata[i].name);
    check(UA_Server_run_startup(server));
    puts("READY");
    fflush(stdout);
    while(running) UA_Server_run_iterate(server, true);
    check(UA_Server_run_shutdown(server));
    UA_Server_delete(server);
    return 0;
}
