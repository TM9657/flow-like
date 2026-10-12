use dust_dds::xtypes::dynamic_type::{
    DynamicDataFactory, DynamicTypeBuilderFactory, ExtensibilityKind, TypeKind,
};

const TYPES_XML_PRIMITIVES: &str = r#"<dds>
    <types>
        <module name="Test">
            <struct name="struct_primitives_final"   extensibility="final">
                <member name="x1"   type="uint8"   />
                <member name="x2"   type="uint16"  />
                <member name="x3"   type="uint32"  />
                <member name="x4"   type="uint64"  />
                <member name="x5"   type="int8"    />
                <member name="x6"   type="int16"   />
                <member name="x7"   type="int32"   />
                <member name="x8"   type="int64"   />
                <member name="x9"   type="boolean" />
                <member name="x10"  type="float32" />
                <member name="x11"  type="float64" />
                <member name="x12"  type="float128"/>
                <member name="x13"  type="byte"    />
                <member name="x14"  type="char8"   />
            </struct>

            <struct name="struct_primitives_appendable"   extensibility="appendable">
                <member name="x1"   type="uint8"   />
                <member name="x2"   type="uint16"  />
                <member name="x3"   type="uint32"  />
                <member name="x4"   type="uint64"  />
                <member name="x5"   type="int8"   />
                <member name="x6"   type="int16"   />
                <member name="x7"   type="int32"   />
                <member name="x8"   type="int64"   />
                <member name="x9"   type="boolean" />
                <member name="x10"  type="float32" />
                <member name="x11"  type="float64" />
                <member name="x12"  type="float128"/>
                <member name="x13"  type="byte"    />
                <member name="x14"  type="char8"   />
            </struct>

            <struct name="struct_primitives_mutable"   extensibility="mutable">
                <member name="x1"   type="uint8"   />
                <member name="x2"   type="uint16"  />
                <member name="x3"   type="uint32"  />
                <member name="x4"   type="uint64"  />
                <member name="x5"   type="int8"   />
                <member name="x6"   type="int16"   />
                <member name="x7"   type="int32"   />
                <member name="x8"   type="int64"   />
                <member name="x9"   type="boolean" />
                <member name="x10"  type="float32" />
                <member name="x11"  type="float64" />
                <member name="x12"  type="float128"/>
                <member name="x13"  type="byte"    />
                <member name="x14"  type="char8"   />
            </struct>

            <union name="union_primitives">
                <discriminator type="uint8"/>
                <case><caseDiscriminator value="0x01"/><member name="x1"  type="uint8"   /></case>
                <case><caseDiscriminator value="0x02"/><member name="x2"  type="uint16"  /></case>
                <case><caseDiscriminator value="0x03"/><member name="x3"  type="uint32"  /></case>
                <case><caseDiscriminator value="0x04"/><member name="x4"  type="uint64"  /></case>
                <case><caseDiscriminator value="0x05"/><member name="x5"  type="int8"    /></case>
                <case><caseDiscriminator value="0x06"/><member name="x6"  type="int16"   /></case>
                <case><caseDiscriminator value="0x07"/><member name="x7"  type="int32"   /></case>
                <case><caseDiscriminator value="0x08"/><member name="x8"  type="int64"   /></case>
                <case><caseDiscriminator value="0x09"/><member name="x9"  type="boolean" /></case>
                <case><caseDiscriminator value="0x0a"/><member name="x10" type="float32" /></case>
                <case><caseDiscriminator value="0x0b"/><member name="x11" type="float64" /></case>
                <case><caseDiscriminator value="0x0c"/><member name="x12" type="float64" /></case>
                <case><caseDiscriminator value="0x0d"/><member name="x13" type="byte"    /></case>
                <case><caseDiscriminator value="0x0e"/><member name="x14" type="char8"   /></case>
            </union>


            <struct name="struct_primitive_uint8"   extensibility="final">
                <member name="x1" type="uint8"/>
            </struct>

            <struct name="struct_primitive_uint16"   extensibility="final">
                <member name="x1"  type="uint16"/>
            </struct>

            <struct name="struct_primitive_uint32"   extensibility="final">
                <member name="x1"  type="uint32"/>
            </struct>

            <struct name="struct_primitive_uint64"   extensibility="final">
                <member name="x1"  type="uint64"/>
            </struct>

            <struct name="struct_primitive_int8"   extensibility="final">
                <member name="x1"  type="int8"/>
            </struct>

            <struct name="struct_primitive_int16"   extensibility="final">
                <member name="x1" type="int16"/>
            </struct>

            <struct name="struct_primitive_int32"   extensibility="final">
                <member name="x1"  type="int32"/>
            </struct>

            <struct name="struct_primitive_int64"   extensibility="final">
                <member name="x1" type="int64"/>
            </struct>

            <struct name="struct_primitive_boolean"   extensibility="final">
                <member name="x1" type="boolean"/>
            </struct>

            <struct name="struct_primitive_float32"   extensibility="final">
                <member name="x1" type="float32"/>
            </struct>

            <struct name="struct_primitive_float64"   extensibility="final">
                <member name="x1" type="float64"/>
            </struct>

            <struct name="struct_primitive_float128"   extensibility="final">
                <member name="x1"  type="float128"/>
            </struct>

            <struct name="struct_primitive_byte"   extensibility="final">
                <member name="x1"  type="byte"/>
            </struct>

            <struct name="struct_primitive_char8"   extensibility="final">
                <member name="x1"  type="char8"/>
            </struct>
        </module>
    </types>
</dds>"#;

const TYPES_XML_ARRAYS: &str = r#"<dds>
    <types>
      <module name="Test">
        <struct name="int32x10"   extensibility="final">
            <member name="x1"   type="int32" arrayDimensions="10"  />
        </struct>

        <struct name="int32x10x2"   extensibility="final">
            <member name="x1"   type="int32" arrayDimensions="10,2"  />
        </struct>

        <struct name="int32x20"   extensibility="final">
            <member name="x1"   type="int32" arrayDimensions="20"  />
        </struct>

        <struct name="uint32x10"   extensibility="final">
            <member name="x1"   type="uint32" arrayDimensions="10"  />
        </struct>

        <struct name="uint32x20"   extensibility="final">
            <member name="x1"   type="uint32" arrayDimensions="20"  />
        </struct>


        <!-- arrays of string types -->
        <struct name="string10x10"   extensibility="final">
            <member name="x1"   type="string" stringMaxLength="10" arrayDimensions="10"  />
        </struct>
        <struct name="string20x10"   extensibility="final">
            <member name="x1"   type="string" stringMaxLength="20" arrayDimensions="10"  />
        </struct>

        <!-- arrays of enum types -->
        <enum name="E1" bitBound="32" extensibility="appendable">
            <enumerator name="VAL0" value="0"/>
            <enumerator name="VAL1" value="1"/>
            <enumerator name="VAL2" value="2"/>
        </enum>

        <enum name="E2" bitBound="32" extensibility="appendable">
            <enumerator name="VAL0" value="0"/>
            <enumerator name="VAL1" value="1"/>
            <enumerator name="VAL2" value="2"/>
            <enumerator name="VAL3" value="3"/>
        </enum>
        <struct name="enum1"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="E1"   />
        </struct>
        <struct name="enum2"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="E2"  />
        </struct>

        <struct name="enum1x10"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="E1" arrayDimensions="10"  />
        </struct>
        <struct name="enum2x10"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="E2" arrayDimensions="10"  />
        </struct>




        <!-- check 'strongly assignable' element type -->
         <!-- F_S__array20_uint32 -->
        <struct name="F_S__array20_uint32"   extensibility="final">
            <member name="x1"   type="uint32" arrayDimensions="20"  />
        </struct>
        <struct name="F_S__array20_uint32_alt"   extensibility="final">
            <member name="altx1"   type="uint32" arrayDimensions="20"  />
        </struct>

        <struct name="A_S__array20_uint32"   extensibility="appendable">
            <member name="x1"   type="uint32" arrayDimensions="20"  />
        </struct>
        <struct name="A_S__array20_uint32_alt"   extensibility="appendable">
            <member name="altx1"   type="uint32" arrayDimensions="20"  />
        </struct>

        <struct name="M_S__array20_uint32"   extensibility="mutable">
            <member name="x1"   type="uint32" arrayDimensions="20"  />
        </struct>
        <struct name="M_S__array20_uint32_alt"   extensibility="mutable">
            <member name="altx1"   type="uint32" arrayDimensions="20"  />
        </struct>

        <!-- F_S__array10_F_S__array20_uint32 -->
        <struct name="F_S__array10_F_S__array20_uint32"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="F_S__array20_uint32" arrayDimensions="10"  />
        </struct>
        <struct name="F_S__array10_F_S__array20_uint32_alt"   extensibility="final">
            <member name="altx1"   type="nonBasic" nonBasicTypeName="F_S__array20_uint32_alt" arrayDimensions="10"  />
        </struct>

        <struct name="F_S__array10_A_S__array20_uint32"   extensibility="final">
            <member name="x1"   type="nonBasic" nonBasicTypeName="A_S__array20_uint32" arrayDimensions="10"  />
        </struct>
        <struct name="F_S__array10_A_S__array20_uint32_alt"   extensibility="final">
            <member name="altx1"   type="nonBasic" nonBasicTypeName="A_S__array20_uint32_alt" arrayDimensions="10"  />
        </struct>

        <struct name="F_S__array10_M_S__array20_uint32"   extensibility="mutable">
            <member name="x1"   type="nonBasic" nonBasicTypeName="M_S__array20_uint32" arrayDimensions="10"  />
        </struct>
        <struct name="F_S__array10_M_S__array20_uint32_alt"   extensibility="mutable">
            <member name="altx1"   type="nonBasic" nonBasicTypeName="M_S__array20_uint32_alt" arrayDimensions="10"  />
        </struct>

      </module>
    </types>
</dds>
"#;

const TYPES_XML_EXTENSIBILITY: &str = r#"<dds>
    <types>
        <module name="Test">
            <struct name="struct_f1"   extensibility="final">
                <member name="x1" type="int32" />
            </struct>
            <struct name="struct_f2"   extensibility="final">
                <member name="x1" type="int32" />
                <member name="x2" type="int32" />
            </struct>

            <struct name="struct_a1"   extensibility="appendable">
                <member name="x1" type="int32" />
            </struct>
            <struct name="struct_a2"   extensibility="appendable">
                <member name="x1" type="int32" />
                <member name="x2" type="int32" />
            </struct>
            <struct name="struct_a3"   extensibility="appendable">
                <member name="x1" type="int32" />
                <member name="x3" type="int32" />
                <member name="x2" type="int32" />
            </struct>

            <struct name="struct_m1"   extensibility="mutable">
                <member name="x1" type="int32" id="1" />
            </struct>
            <struct name="struct_m2"   extensibility="mutable">
                <member name="x1" type="int32" id="1" />
                <member name="x2" type="int32" id="2" />
            </struct>
            <struct name="struct_m3"   extensibility="mutable">
                <member name="x1" type="int32" id="1" />
                <member name="x3" type="int32" id="3" />
                <member name="x2" type="int32" id="2" />
            </struct>
            <struct name="struct_m4"   extensibility="mutable">
                <member name="x1" type="int32" />
                <member name="x3" type="int32" />
                <member name="x2" type="int32" />
            </struct>

            <struct name="struct_hashid_1" extensibility="final" autoid="hash">
                <member name="x1" type="int32"/>
            </struct>
            <struct name="struct_hashid_2" extensibility="final" autoid="hash">
                <member name="x2" type="int32" hashid="x1"/>
            </struct>
        </module>
    </types>
</dds>"#;

const TYPES_XML_TRY_CONSTRUCT: &str = r#"
<dds>
     <types>
        <module name="Test">
            <struct name="seq_int32x20"  extensibility="final">
                <member name="x1"   type="int32" sequenceMaxLength="20"  />
            </struct>
            <struct name="seq_int32x10_trim"   extensibility="final">
                <member name="x1"   type="int32" sequenceMaxLength="10" tryConstruct="trim"/>
            </struct>
            <struct name="seq_int32x10_discard" extensibility="final">
                <member name="x1"   type="int32" sequenceMaxLength="10" tryConstruct="discard"/>
            </struct>
            <struct name="seq_int32x10_default"   extensibility="final">
                <member name="x1"   type="int32" sequenceMaxLength="10" tryConstruct="use_default"/>
            </struct>
            <union name="union_seq_int32x20"> <discriminator type="uint32" />
                <case><caseDiscriminator value="1" /><member name="x1"   type="int32" sequenceMaxLength="20" /></case>
            </union>
            <union name="union_seq_int32x10_trim"> <discriminator type="uint32" />
                <case><caseDiscriminator value="1" /><member name="x1"   type="int32" sequenceMaxLength="10" tryConstruct="trim"/></case>
            </union>
        </module>
    </types>
</dds>"#;

const DATA_XML_STRUCT_PRIMITIVES: &str = r#"<struct_primitives>
  <x1>0x01</x1>
  <x2>2</x2>
  <x3>3</x3>
  <x4>4</x4>
  <x5>0x05</x5>
  <x6>6</x6>
  <x7>7</x7>
  <x8>8</x8>
  <x9>true</x9>
  <x10>10.100000</x10>
  <x11>11.200000</x11>
  <x12>12.300000</x12>
  <x13>13</x13>
  <x14>0x0e</x14>
</struct_primitives>"#;

const DATA_XML_ARRAY_NUM_10: &str = r#"<struct>
  <x1>
    <item>1</item>
    <item>2</item>
    <item>3</item>
    <item>4</item>
    <item>5</item>
    <item>6</item>
    <item>7</item>
    <item>8</item>
    <item>9</item>
    <item>10</item>
  </x1>
</struct>
"#;

const DATA_XML_ARRAY_ENUM_10: &str = r#"
<struct>
  <x1>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
    <item>VAL1</item>
  </x1>
</struct>
"#;

const DATA_XML_ENUM_SINGLE: &str = r#"
<struct>
  <x1>VAL1</x1>
</struct>
"#;

const DATA_XML_ARRAY_NUM_20: &str = r#"
<struct>
  <x1>
    <item>1</item>
    <item>2</item>
    <item>3</item>
    <item>4</item>
    <item>5</item>
    <item>6</item>
    <item>7</item>
    <item>8</item>
    <item>9</item>
    <item>10</item>
    <item>11</item>
    <item>12</item>
    <item>13</item>
    <item>14</item>
    <item>15</item>
    <item>16</item>
    <item>17</item>
    <item>18</item>
    <item>19</item>
    <item>20</item>
  </x1>
</struct>
"#;

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_primitive_int8_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::struct_primitive_int8",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    assert_eq!(ty.get_member_count(), 1);
    let member = ty.get_member_by_name("x1").unwrap();
    assert_eq!(member.descriptor.r#type.get_kind(), TypeKind::INT8);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_primitives_final_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::struct_primitives_final",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    assert_eq!(
        ty.get_descriptor().extensibility_kind,
        ExtensibilityKind::Final
    );
    assert_eq!(ty.get_member_count(), 14);

    let m1 = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m1.descriptor.r#type.get_kind(), TypeKind::UINT8);

    let m2 = ty.get_member_by_name("x2").unwrap();
    assert_eq!(m2.descriptor.r#type.get_kind(), TypeKind::UINT16);

    let m3 = ty.get_member_by_name("x3").unwrap();
    assert_eq!(m3.descriptor.r#type.get_kind(), TypeKind::UINT32);

    let m4 = ty.get_member_by_name("x4").unwrap();
    assert_eq!(m4.descriptor.r#type.get_kind(), TypeKind::UINT64);

    let m5 = ty.get_member_by_name("x5").unwrap();
    assert_eq!(m5.descriptor.r#type.get_kind(), TypeKind::INT8);

    let m6 = ty.get_member_by_name("x6").unwrap();
    assert_eq!(m6.descriptor.r#type.get_kind(), TypeKind::INT16);

    let m7 = ty.get_member_by_name("x7").unwrap();
    assert_eq!(m7.descriptor.r#type.get_kind(), TypeKind::INT32);

    let m8 = ty.get_member_by_name("x8").unwrap();
    assert_eq!(m8.descriptor.r#type.get_kind(), TypeKind::INT64);

    let m9 = ty.get_member_by_name("x9").unwrap();
    assert_eq!(m9.descriptor.r#type.get_kind(), TypeKind::BOOLEAN);

    let m10 = ty.get_member_by_name("x10").unwrap();
    assert_eq!(m10.descriptor.r#type.get_kind(), TypeKind::FLOAT32);

    let m11 = ty.get_member_by_name("x11").unwrap();
    assert_eq!(m11.descriptor.r#type.get_kind(), TypeKind::FLOAT64);

    let m12 = ty.get_member_by_name("x12").unwrap();
    assert_eq!(m12.descriptor.r#type.get_kind(), TypeKind::FLOAT128);

    let m13 = ty.get_member_by_name("x13").unwrap();
    assert_eq!(m13.descriptor.r#type.get_kind(), TypeKind::BYTE);

    let m14 = ty.get_member_by_name("x14").unwrap();
    assert_eq!(m14.descriptor.r#type.get_kind(), TypeKind::CHAR8);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn struct_primitives_final_data_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::struct_primitives_final",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(DATA_XML_STRUCT_PRIMITIVES).unwrap();

    assert_eq!(d.get_uint8_value(0).unwrap(), &1);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn struct_primitive_float128_data_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::struct_primitive_float128",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml("<struct_primitive_float128><x1>0.5</x1></struct_primitive_float128>")
        .unwrap();

    let expected_f128_half: i128 = 0x3ffe_0000_0000_0000_0000_0000_0000_0000_u128 as i128;
    assert_eq!(d.get_float128_value(0).unwrap(), &expected_f128_half);
    assert_eq!(
        expected_f128_half.to_le_bytes(),
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xfe, 0x3f]
    );

    let mut d2 = DynamicDataFactory::create_data(ty);
    d2.from_xml("<struct_primitive_float128><x1>12.300000</x1></struct_primitive_float128>")
        .unwrap();

    let expected_f128_12_3: i128 = 0x4002_8999_9999_9999_9999_9999_9999_999a_u128 as i128;
    assert_eq!(d2.get_float128_value(0).unwrap(), &expected_f128_12_3);
    assert_eq!(
        expected_f128_12_3.to_le_bytes(),
        [
            0x9a, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x99, 0x89,
            0x02, 0x40
        ]
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_primitives_appendable_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::struct_primitives_appendable",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    assert_eq!(
        ty.get_descriptor().extensibility_kind,
        ExtensibilityKind::Appendable
    );
    assert_eq!(ty.get_member_count(), 14);

    let m1 = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m1.descriptor.r#type.get_kind(), TypeKind::UINT8);

    let m2 = ty.get_member_by_name("x2").unwrap();
    assert_eq!(m2.descriptor.r#type.get_kind(), TypeKind::UINT16);

    let m3 = ty.get_member_by_name("x3").unwrap();
    assert_eq!(m3.descriptor.r#type.get_kind(), TypeKind::UINT32);

    let m4 = ty.get_member_by_name("x4").unwrap();
    assert_eq!(m4.descriptor.r#type.get_kind(), TypeKind::UINT64);

    let m5 = ty.get_member_by_name("x5").unwrap();
    assert_eq!(m5.descriptor.r#type.get_kind(), TypeKind::INT8);

    let m6 = ty.get_member_by_name("x6").unwrap();
    assert_eq!(m6.descriptor.r#type.get_kind(), TypeKind::INT16);

    let m7 = ty.get_member_by_name("x7").unwrap();
    assert_eq!(m7.descriptor.r#type.get_kind(), TypeKind::INT32);

    let m8 = ty.get_member_by_name("x8").unwrap();
    assert_eq!(m8.descriptor.r#type.get_kind(), TypeKind::INT64);

    let m9 = ty.get_member_by_name("x9").unwrap();
    assert_eq!(m9.descriptor.r#type.get_kind(), TypeKind::BOOLEAN);

    let m10 = ty.get_member_by_name("x10").unwrap();
    assert_eq!(m10.descriptor.r#type.get_kind(), TypeKind::FLOAT32);

    let m11 = ty.get_member_by_name("x11").unwrap();
    assert_eq!(m11.descriptor.r#type.get_kind(), TypeKind::FLOAT64);

    let m12 = ty.get_member_by_name("x12").unwrap();
    assert_eq!(m12.descriptor.r#type.get_kind(), TypeKind::FLOAT128);

    let m13 = ty.get_member_by_name("x13").unwrap();
    assert_eq!(m13.descriptor.r#type.get_kind(), TypeKind::BYTE);

    let m14 = ty.get_member_by_name("x14").unwrap();
    assert_eq!(m14.descriptor.r#type.get_kind(), TypeKind::CHAR8);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_union_primitives_final_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::union_primitives",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    assert_eq!(ty.get_kind(), TypeKind::UNION);
    assert_eq!(
        ty.get_descriptor().extensibility_kind,
        ExtensibilityKind::Appendable
    );
    assert_eq!(
        ty.get_descriptor().discriminator_type.unwrap().get_kind(),
        TypeKind::UINT8
    );
    assert_eq!(ty.get_member_count(), 15);

    let m_discriminator = ty.get_member_by_name("discriminator").unwrap();
    assert_eq!(
        m_discriminator.descriptor.r#type.get_kind(),
        TypeKind::UINT8
    );
    assert!(m_discriminator.descriptor.is_must_understand);

    let m1 = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m1.descriptor.r#type.get_kind(), TypeKind::UINT8);
    assert_eq!(m1.descriptor.label, &[1]);

    let m2 = ty.get_member_by_name("x2").unwrap();
    assert_eq!(m2.descriptor.r#type.get_kind(), TypeKind::UINT16);
    assert_eq!(m2.descriptor.label, &[2]);

    let m3 = ty.get_member_by_name("x3").unwrap();
    assert_eq!(m3.descriptor.r#type.get_kind(), TypeKind::UINT32);
    assert_eq!(m3.descriptor.label, &[3]);

    let m4 = ty.get_member_by_name("x4").unwrap();
    assert_eq!(m4.descriptor.r#type.get_kind(), TypeKind::UINT64);
    assert_eq!(m4.descriptor.label, &[4]);

    let m5 = ty.get_member_by_name("x5").unwrap();
    assert_eq!(m5.descriptor.r#type.get_kind(), TypeKind::INT8);
    assert_eq!(m5.descriptor.label, &[5]);

    let m6 = ty.get_member_by_name("x6").unwrap();
    assert_eq!(m6.descriptor.r#type.get_kind(), TypeKind::INT16);
    assert_eq!(m6.descriptor.label, &[6]);

    let m7 = ty.get_member_by_name("x7").unwrap();
    assert_eq!(m7.descriptor.r#type.get_kind(), TypeKind::INT32);
    assert_eq!(m7.descriptor.label, &[7]);

    let m8 = ty.get_member_by_name("x8").unwrap();
    assert_eq!(m8.descriptor.r#type.get_kind(), TypeKind::INT64);
    assert_eq!(m8.descriptor.label, &[8]);

    let m9 = ty.get_member_by_name("x9").unwrap();
    assert_eq!(m9.descriptor.r#type.get_kind(), TypeKind::BOOLEAN);
    assert_eq!(m9.descriptor.label, &[9]);

    let m10 = ty.get_member_by_name("x10").unwrap();
    assert_eq!(m10.descriptor.r#type.get_kind(), TypeKind::FLOAT32);
    assert_eq!(m10.descriptor.label, &[10]);

    let m11 = ty.get_member_by_name("x11").unwrap();
    assert_eq!(m11.descriptor.r#type.get_kind(), TypeKind::FLOAT64);
    assert_eq!(m11.descriptor.label, &[11]);

    let m12 = ty.get_member_by_name("x12").unwrap();
    assert_eq!(m12.descriptor.r#type.get_kind(), TypeKind::FLOAT64);
    assert_eq!(m12.descriptor.label, &[12]);

    let m13 = ty.get_member_by_name("x13").unwrap();
    assert_eq!(m13.descriptor.r#type.get_kind(), TypeKind::BYTE);
    assert_eq!(m13.descriptor.label, &[13]);

    let m14 = ty.get_member_by_name("x14").unwrap();
    assert_eq!(m14.descriptor.r#type.get_kind(), TypeKind::CHAR8);
    assert_eq!(m14.descriptor.label, &[14]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn union_primitives_final_data_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::union_primitives",
        vec![],
    )
    .unwrap();

    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(
        "<union_primitives>
            <discriminator>0x05</discriminator>
            <x5>5</x5>
        </union_primitives>",
    )
    .unwrap();

    assert_eq!(d.get_uint8_value(0).unwrap(), &0x05);
    assert_eq!(d.get_int8_value(5).unwrap(), &5);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn union_primitives_final_data_from_xml_without_explicit_discriminator() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_PRIMITIVES,
        "Test::union_primitives",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(
        "<struct>
            <x5>5</x5>
        </struct>",
    )
    .unwrap();

    assert_eq!(d.get_uint8_value(0).unwrap(), &0x05);
    assert_eq!(d.get_int8_value(5).unwrap(), &5);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn data_from_xml_array_num_10() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::int32x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(DATA_XML_ARRAY_NUM_10).unwrap();

    let expected_values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(d.get_int32_values(0).unwrap(), expected_values.as_slice());
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn data_from_xml_array_enum_10() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::enum1x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(DATA_XML_ARRAY_ENUM_10).unwrap();

    let values = d.get_complex_values(0).unwrap();
    assert_eq!(values.len(), 10);
    for value in values {
        assert_eq!(value.get_int32_value(0).unwrap(), &1);
    }
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn data_from_xml_enum_single_from_test_enum1() {
    let builder =
        DynamicTypeBuilderFactory::create_type_w_document(TYPES_XML_ARRAYS, "Test::enum1", vec![])
            .unwrap();
    let ty = builder.build();

    let mut d = DynamicDataFactory::create_data(ty);
    d.from_xml(DATA_XML_ENUM_SINGLE).unwrap();

    assert_eq!(
        d.get_complex_value(0).unwrap().get_int32_value(0).unwrap(),
        &1
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_int32x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::int32x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    assert_eq!(
        m.descriptor
            .r#type
            .get_descriptor()
            .element_type
            .unwrap()
            .get_kind(),
        TypeKind::INT32
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_int32x10x2_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::int32x10x2",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10, 2]);
    let element_type = m.descriptor.r#type.get_descriptor().element_type.unwrap();
    assert_eq!(element_type.get_kind(), TypeKind::INT32);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_int32x20_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::int32x20",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_uint32x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::uint32x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    assert_eq!(
        m.descriptor
            .r#type
            .get_descriptor()
            .element_type
            .unwrap()
            .get_kind(),
        TypeKind::UINT32
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_uint32x20_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::uint32x20",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_string10x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::string10x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    let inner = m.descriptor.r#type.get_descriptor().element_type.unwrap();
    assert_eq!(inner.get_kind(), TypeKind::STRING8);
    assert_eq!(inner.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_string20x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::string20x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    let inner = m.descriptor.r#type.get_descriptor().element_type.unwrap();
    assert_eq!(inner.get_kind(), TypeKind::STRING8);
    assert_eq!(inner.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_enum1_from_xml() {
    let builder =
        DynamicTypeBuilderFactory::create_type_w_document(TYPES_XML_ARRAYS, "Test::enum1", vec![])
            .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    // Enum is tested by parsing the struct member 'enum1' with nonBasicTypeName="E1"
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ENUM);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_enum2_from_xml() {
    let builder =
        DynamicTypeBuilderFactory::create_type_w_document(TYPES_XML_ARRAYS, "Test::enum2", vec![])
            .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ENUM);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_enum1x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::enum1x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    assert_eq!(
        m.descriptor
            .r#type
            .get_descriptor()
            .element_type
            .unwrap()
            .get_kind(),
        TypeKind::ENUM
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_enum2x10_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::enum2x10",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    assert_eq!(
        m.descriptor
            .r#type
            .get_descriptor()
            .element_type
            .unwrap()
            .get_kind(),
        TypeKind::ENUM
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_a_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::A_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_a_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::A_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_m_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::M_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_m_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::M_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[20]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_f_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_F_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
    // The nested type is a struct with an array
    assert_eq!(
        m.descriptor
            .r#type
            .get_descriptor()
            .element_type
            .unwrap()
            .get_kind(),
        TypeKind::STRUCTURE
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_f_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_F_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_a_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_A_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_a_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_A_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_m_s_array20_uint32_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_M_S__array20_uint32",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_f_s_array10_m_s_array20_uint32_alt_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_ARRAYS,
        "Test::F_S__array10_M_S__array20_uint32_alt",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    let m = ty.get_member_by_name("altx1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::ARRAY);
    assert_eq!(m.descriptor.r#type.get_descriptor().bound, &[10]);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_m1_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_EXTENSIBILITY,
        "Test::struct_m1",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    assert_eq!(ty.get_kind(), TypeKind::STRUCTURE);
    assert_eq!(ty.descriptor.extensibility_kind, ExtensibilityKind::Mutable);
    let m = ty.get_member_by_name("x1").unwrap();
    assert_eq!(m.descriptor.r#type.get_kind(), TypeKind::INT32);
    assert_eq!(m.descriptor.id, 1);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_m3_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_EXTENSIBILITY,
        "Test::struct_m3",
        vec![],
    )
    .unwrap();
    let ty = builder.build();
    assert_eq!(ty.get_kind(), TypeKind::STRUCTURE);
    assert_eq!(ty.descriptor.extensibility_kind, ExtensibilityKind::Mutable);
    let memember_x1 = ty.get_member_by_name("x1").unwrap();
    assert_eq!(memember_x1.descriptor.r#type.get_kind(), TypeKind::INT32);
    assert_eq!(memember_x1.descriptor.id, 1);
    let memember_x2 = ty.get_member_by_name("x2").unwrap();
    assert_eq!(memember_x2.descriptor.r#type.get_kind(), TypeKind::INT32);
    assert_eq!(memember_x2.descriptor.id, 2);
    let memember_x3 = ty.get_member_by_name("x3").unwrap();
    assert_eq!(memember_x3.descriptor.r#type.get_kind(), TypeKind::INT32);
    assert_eq!(memember_x3.descriptor.id, 3);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_seq_int32x10_from_xml() {
    use dust_dds::xtypes::dynamic_type::TryConstructKind;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::seq_int32x20",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x1").unwrap();
    assert_eq!(
        member.descriptor.try_construct_kind,
        TryConstructKind::Discard
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_seq_int32x10_trim_from_xml() {
    use dust_dds::xtypes::dynamic_type::TryConstructKind;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::seq_int32x10_trim",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x1").unwrap();
    assert_eq!(member.descriptor.try_construct_kind, TryConstructKind::Trim);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_seq_int32x10_discard_from_xml() {
    use dust_dds::xtypes::dynamic_type::TryConstructKind;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::seq_int32x10_discard",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x1").unwrap();
    assert_eq!(
        member.descriptor.try_construct_kind,
        TryConstructKind::Discard
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_seq_int32x10_default_from_xml() {
    use dust_dds::xtypes::dynamic_type::TryConstructKind;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::seq_int32x10_default",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x1").unwrap();
    assert_eq!(
        member.descriptor.try_construct_kind,
        TryConstructKind::UseDefault
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_union_seq_int32x10_trim_from_xml() {
    use dust_dds::xtypes::dynamic_type::TryConstructKind;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::union_seq_int32x10_trim",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    assert_eq!(dynamic_type.get_kind(), TypeKind::UNION);
    assert_eq!(
        dynamic_type
            .get_descriptor()
            .discriminator_type
            .unwrap()
            .get_kind(),
        TypeKind::UINT32
    );
    assert_eq!(dynamic_type.get_member_count(), 2);
    let member = dynamic_type.get_member_by_name("x1").unwrap();

    assert_eq!(member.descriptor.r#type.descriptor.kind, TypeKind::SEQUENCE);
    assert_eq!(
        member
            .descriptor
            .r#type
            .descriptor
            .element_type
            .unwrap()
            .descriptor
            .kind,
        TypeKind::INT32
    );
    assert_eq!(member.descriptor.try_construct_kind, TryConstructKind::Trim);

    let mut d = DynamicDataFactory::create_data(dynamic_type);
    d.from_xml(DATA_XML_ARRAY_NUM_20).unwrap();

    // Discriminator:
    assert_eq!(d.get_uint32_value(0).unwrap(), &1);
    assert_eq!(
        d.get_int32_values(1).unwrap(),
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
        ]
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn data_union_seq_int32x10_trim_from_xml() {
    let dynamic_type = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::union_seq_int32x10_trim",
        vec![],
    )
    .unwrap()
    .build();
    let mut d = DynamicDataFactory::create_data(dynamic_type);
    d.from_xml(DATA_XML_ARRAY_NUM_10).unwrap();
    assert_eq!(
        d.get_int32_values(1).unwrap(),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn data_union_seq_int32x20_from_xml() {
    let dynamic_type = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_TRY_CONSTRUCT,
        "Test::union_seq_int32x20",
        vec![],
    )
    .unwrap()
    .build();
    let mut d = DynamicDataFactory::create_data(dynamic_type);
    d.from_xml(DATA_XML_ARRAY_NUM_20).unwrap();
    assert_eq!(
        d.get_int32_values(1).unwrap(),
        &[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
        ]
    );
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn parse_must_understand_attribute_from_xml() {
    let type_xml = r#"<dds>
        <types>
            <module name="Test">
                <struct name="struct_mustUnderstand" extensibility="appendable">
                    <member name="x1" type="int32"/>
                    <member name="x2" mustUnderstand="true" type="int32"/>
                </struct>
            </module>
        </types>
    </dds>"#;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        type_xml,
        "Test::struct_mustUnderstand",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member_x1 = dynamic_type.get_member_by_name("x1").unwrap();
    assert!(!member_x1.descriptor.is_must_understand);

    let member_x2 = dynamic_type.get_member_by_name("x2").unwrap();
    assert!(member_x2.descriptor.is_must_understand);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn parse_bitmask_type_and_data_from_xml() {
    let type_xml = r#"<dds>
        <types>
            <module name="Test">
                <bitmask name="B_32" bitBound="32">
                    <flag name="B_FLAG_1" value="0"/>
                    <flag name="B_FLAG_2" value="1"/>
                </bitmask>
                <union name="union_bitmask32" extensibility="appendable">
                    <discriminator type="nonBasic" nonBasicTypeName="B_32"/>
                    <case><caseDiscriminator value="2"/><member name="x1" type="int16"/></case>
                    <case><caseDiscriminator value="1"/><member name="x2" type="int32"/></case>
                </union>
            </module>
        </types>
    </dds>"#;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        type_xml,
        "Test::union_bitmask32",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    assert_eq!(dynamic_type.get_kind(), TypeKind::UNION);

    let disc_member = dynamic_type.get_member(0).unwrap();
    assert_eq!(disc_member.descriptor.r#type.get_kind(), TypeKind::BITMASK);
    assert_eq!(
        disc_member.descriptor.r#type.get_descriptor().bound.first(),
        Some(&32)
    );

    let mut data = DynamicDataFactory::create_data(dynamic_type);
    let data_xml = r#"<union_bitmask>
        <discriminator>2</discriminator>
        <x1>128</x1>
    </union_bitmask>"#;
    data.from_xml(data_xml).unwrap();
    assert_eq!(*data.get_uint32_value(0).unwrap(), 2);
    assert_eq!(*data.get_int16_value(1).unwrap(), 128);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn union_appendable_a_default_from_xml() {
    let type_xml = r#"<dds>
        <types>
            <module name="Test">
                <union name="union_appendable_a_default" extensibility="appendable">
                    <discriminator type="uint32" />
                    <case><caseDiscriminator value="5"/><member name="x5" type="uint32"/></case>
                    <case><caseDiscriminator value="default"/><member name="xd" type="uint32"/></case>
                </union>
            </module>
        </types>
    </dds>"#;

    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        type_xml,
        "Test::union_appendable_a_default",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    assert_eq!(dynamic_type.get_kind(), TypeKind::UNION);

    let mut data = DynamicDataFactory::create_data(dynamic_type);
    let data_xml = r#"<union_xd>
        <discriminator>10</discriminator>
        <xd>12345</xd>
    </union_xd>"#;
    data.from_xml(data_xml).unwrap();
    assert_eq!(*data.get_uint32_value(0).unwrap(), 10);
    assert_eq!(*data.get_uint32_value(2).unwrap(), 12345);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_hashid_1_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_EXTENSIBILITY,
        "Test::struct_hashid_1",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x1").unwrap();
    let hash = md5::compute(b"x1");
    let expected_id = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) & 0x0FFF_FFFF;
    assert_eq!(member.get_id(), expected_id);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn create_struct_hashid_2_from_xml() {
    let builder = DynamicTypeBuilderFactory::create_type_w_document(
        TYPES_XML_EXTENSIBILITY,
        "Test::struct_hashid_2",
        vec![],
    )
    .unwrap();
    let dynamic_type = builder.build();
    let member = dynamic_type.get_member_by_name("x2").unwrap();
    let hash = md5::compute(b"x1");
    let expected_id = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) & 0x0FFF_FFFF;
    assert_eq!(member.get_id(), expected_id);
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn convert_dynamic_type_to_complete_type_object() {
    use dust_dds::xtypes::type_object::{CompleteTypeObject, TYPE_FLAG_IS_FINAL, TypeIdentifier};

    let type_xml = r#"
    <dds>
        <types>
            <module name="Test">
                <enum name="E1" bitBound="32" extensibility="appendable">
                    <enumerator name="VAL0" value="0"/>
                    <enumerator name="VAL1" value="1"/>
                    <enumerator name="VAL2" value="2"/>
                </enum>
                <enum name="E2" bitBound="32" extensibility="appendable">
                    <enumerator name="VAL0" value="0"/>
                    <enumerator name="VAL1" value="1"/>
                    <enumerator name="VAL2" value="2"/>
                    <enumerator name="VAL3" value="3"/>
                </enum>
                <struct name="enum1x10"   extensibility="final">
                    <member name="x1"   type="nonBasic" nonBasicTypeName="E1" arrayDimensions="10"  />
                </struct>
                <struct name="enum2x10"   extensibility="final">
                    <member name="x1"   type="nonBasic" nonBasicTypeName="E2" arrayDimensions="10"  />
                </struct>
            </module>
        </types>
    </dds>
    "#;
    let publisher_dynamic_type =
        DynamicTypeBuilderFactory::create_type_w_document(type_xml, "Test::enum1x10", vec![])
            .unwrap()
            .build();

    let complete_type: CompleteTypeObject = CompleteTypeObject::from(publisher_dynamic_type);

    match complete_type {
        CompleteTypeObject::TkStructure { struct_type } => {
            assert_eq!(struct_type.header.detail.type_name, "Test::enum1x10");
            assert_eq!(
                struct_type.struct_flags & TYPE_FLAG_IS_FINAL,
                TYPE_FLAG_IS_FINAL
            );
            assert_eq!(struct_type.member_seq.len(), 1);

            let member = &struct_type.member_seq[0];
            assert_eq!(member.detail.name, "x1");
            assert_eq!(member.common.member_id, 0);

            match &member.common.member_type_id {
                TypeIdentifier::TiPlainArraySmall { array_sdefn } => {
                    assert_eq!(array_sdefn.array_bound_seq, vec![10]);
                    assert!(matches!(
                        *array_sdefn.element_identifier,
                        TypeIdentifier::EkComplete { .. }
                    ));
                }
                TypeIdentifier::TiPlainArrayLarge { array_ldefn } => {
                    assert_eq!(array_ldefn.array_bound_seq, vec![10]);
                    assert!(matches!(
                        *array_ldefn.element_identifier,
                        TypeIdentifier::EkComplete { .. }
                    ));
                }
                _ => panic!("Expected array type identifier for member x1"),
            }
        }
        _ => panic!("Expected TkStructure complete type object"),
    }
}

#[cfg(feature = "xtypes-xml")]
#[test]
fn convert_enum_dynamic_type_to_complete_type_object() {
    use dust_dds::xtypes::type_object::CompleteTypeObject;

    let type_xml = r#"
    <dds>
        <types>
            <module name="Test">
                <enum name="E1" bitBound="32" extensibility="appendable">
                    <enumerator name="VAL0" value="0"/>
                    <enumerator name="VAL1" value="1"/>
                    <enumerator name="VAL2" value="2"/>
                </enum>
                <enum name="E2" bitBound="32" extensibility="appendable">
                    <enumerator name="VAL0" value="0"/>
                    <enumerator name="VAL1" value="1"/>
                    <enumerator name="VAL2" value="2"/>
                    <enumerator name="VAL3" value="3"/>
                </enum>
                <struct name="enum1x10"   extensibility="final">
                    <member name="x1"   type="nonBasic" nonBasicTypeName="E1" arrayDimensions="10"  />
                </struct>
                <struct name="enum2x10"   extensibility="final">
                    <member name="x1"   type="nonBasic" nonBasicTypeName="E2" arrayDimensions="10"  />
                </struct>
            </module>
        </types>
    </dds>
    "#;

    let e1_dynamic_type =
        DynamicTypeBuilderFactory::create_type_w_document(type_xml, "Test::E1", vec![])
            .unwrap()
            .build();

    let complete_type = CompleteTypeObject::from(e1_dynamic_type);

    match complete_type {
        CompleteTypeObject::TkEnum { enumerated_type } => {
            assert_eq!(enumerated_type.header.detail.type_name, "Test::E1");
            assert_eq!(enumerated_type.literal_seq.len(), 3);
            assert_eq!(enumerated_type.literal_seq[0].detail.name, "VAL0");
            assert_eq!(enumerated_type.literal_seq[0].common.value, 0);
            assert_eq!(enumerated_type.literal_seq[1].detail.name, "VAL1");
            assert_eq!(enumerated_type.literal_seq[1].common.value, 1);
            assert_eq!(enumerated_type.literal_seq[2].detail.name, "VAL2");
            assert_eq!(enumerated_type.literal_seq[2].common.value, 2);
        }
        _ => panic!("Expected TkEnum complete type object"),
    }
}
