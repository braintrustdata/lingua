impl serde::Serialize for AttributionSourceId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.source.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId", len)?;
        if let Some(v) = self.source.as_ref() {
            match v {
                attribution_source_id::Source::GroundingPassage(v) => {
                    struct_ser.serialize_field("groundingPassage", v)?;
                }
                attribution_source_id::Source::SemanticRetrieverChunk(v) => {
                    struct_ser.serialize_field("semanticRetrieverChunk", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AttributionSourceId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "grounding_passage",
            "groundingPassage",
            "semantic_retriever_chunk",
            "semanticRetrieverChunk",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            GroundingPassage,
            SemanticRetrieverChunk,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "groundingPassage" | "grounding_passage" => Ok(GeneratedField::GroundingPassage),
                            "semanticRetrieverChunk" | "semantic_retriever_chunk" => Ok(GeneratedField::SemanticRetrieverChunk),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AttributionSourceId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.AttributionSourceId")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AttributionSourceId, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut source__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::GroundingPassage => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingPassage"));
                            }
                            source__ = map_.next_value::<::std::option::Option<_>>()?.map(attribution_source_id::Source::GroundingPassage)
;
                        }
                        GeneratedField::SemanticRetrieverChunk => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("semanticRetrieverChunk"));
                            }
                            source__ = map_.next_value::<::std::option::Option<_>>()?.map(attribution_source_id::Source::SemanticRetrieverChunk)
;
                        }
                    }
                }
                Ok(AttributionSourceId {
                    source: source__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for attribution_source_id::GroundingPassageId {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.passage_id.is_empty() {
            len += 1;
        }
        if self.part_index != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId.GroundingPassageId", len)?;
        if !self.passage_id.is_empty() {
            struct_ser.serialize_field("passageId", &self.passage_id)?;
        }
        if self.part_index != 0 {
            struct_ser.serialize_field("partIndex", &self.part_index)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for attribution_source_id::GroundingPassageId {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "passage_id",
            "passageId",
            "part_index",
            "partIndex",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PassageId,
            PartIndex,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "passageId" | "passage_id" => Ok(GeneratedField::PassageId),
                            "partIndex" | "part_index" => Ok(GeneratedField::PartIndex),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = attribution_source_id::GroundingPassageId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.AttributionSourceId.GroundingPassageId")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<attribution_source_id::GroundingPassageId, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut passage_id__ = None;
                let mut part_index__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PassageId => {
                            if passage_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("passageId"));
                            }
                            passage_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PartIndex => {
                            if part_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("partIndex"));
                            }
                            part_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(attribution_source_id::GroundingPassageId {
                    passage_id: passage_id__.unwrap_or_default(),
                    part_index: part_index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId.GroundingPassageId", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for attribution_source_id::SemanticRetrieverChunk {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.source.is_empty() {
            len += 1;
        }
        if !self.chunk.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId.SemanticRetrieverChunk", len)?;
        if !self.source.is_empty() {
            struct_ser.serialize_field("source", &self.source)?;
        }
        if !self.chunk.is_empty() {
            struct_ser.serialize_field("chunk", &self.chunk)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for attribution_source_id::SemanticRetrieverChunk {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "source",
            "chunk",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Source,
            Chunk,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "source" => Ok(GeneratedField::Source),
                            "chunk" => Ok(GeneratedField::Chunk),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = attribution_source_id::SemanticRetrieverChunk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.AttributionSourceId.SemanticRetrieverChunk")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<attribution_source_id::SemanticRetrieverChunk, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut source__ = None;
                let mut chunk__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Source => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("source"));
                            }
                            source__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Chunk => {
                            if chunk__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chunk"));
                            }
                            chunk__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(attribution_source_id::SemanticRetrieverChunk {
                    source: source__.unwrap_or_default(),
                    chunk: chunk__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.AttributionSourceId.SemanticRetrieverChunk", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AudioTranscriptionConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.AudioTranscriptionConfig", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AudioTranscriptionConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AudioTranscriptionConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.AudioTranscriptionConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AudioTranscriptionConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(AudioTranscriptionConfig {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.AudioTranscriptionConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BatchEmbedContentsRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if !self.requests.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BatchEmbedContentsRequest", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if !self.requests.is_empty() {
            struct_ser.serialize_field("requests", &self.requests)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BatchEmbedContentsRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "requests",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            Requests,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "requests" => Ok(GeneratedField::Requests),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BatchEmbedContentsRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BatchEmbedContentsRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BatchEmbedContentsRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut requests__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Requests => {
                            if requests__.is_some() {
                                return Err(serde::de::Error::duplicate_field("requests"));
                            }
                            requests__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BatchEmbedContentsRequest {
                    model: model__.unwrap_or_default(),
                    requests: requests__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BatchEmbedContentsRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BatchEmbedContentsResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.embeddings.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BatchEmbedContentsResponse", len)?;
        if !self.embeddings.is_empty() {
            struct_ser.serialize_field("embeddings", &self.embeddings)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BatchEmbedContentsResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "embeddings",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Embeddings,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "embeddings" => Ok(GeneratedField::Embeddings),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BatchEmbedContentsResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BatchEmbedContentsResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BatchEmbedContentsResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut embeddings__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Embeddings => {
                            if embeddings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("embeddings"));
                            }
                            embeddings__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BatchEmbedContentsResponse {
                    embeddings: embeddings__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BatchEmbedContentsResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentClientContent {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.turns.is_empty() {
            len += 1;
        }
        if self.turn_complete {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentClientContent", len)?;
        if !self.turns.is_empty() {
            struct_ser.serialize_field("turns", &self.turns)?;
        }
        if self.turn_complete {
            struct_ser.serialize_field("turnComplete", &self.turn_complete)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentClientContent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "turns",
            "turn_complete",
            "turnComplete",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Turns,
            TurnComplete,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "turns" => Ok(GeneratedField::Turns),
                            "turnComplete" | "turn_complete" => Ok(GeneratedField::TurnComplete),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentClientContent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentClientContent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentClientContent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut turns__ = None;
                let mut turn_complete__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Turns => {
                            if turns__.is_some() {
                                return Err(serde::de::Error::duplicate_field("turns"));
                            }
                            turns__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TurnComplete => {
                            if turn_complete__.is_some() {
                                return Err(serde::de::Error::duplicate_field("turnComplete"));
                            }
                            turn_complete__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentClientContent {
                    turns: turns__.unwrap_or_default(),
                    turn_complete: turn_complete__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentClientContent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentClientMessage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.message_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentClientMessage", len)?;
        if let Some(v) = self.message_type.as_ref() {
            match v {
                bidi_generate_content_client_message::MessageType::Setup(v) => {
                    struct_ser.serialize_field("setup", v)?;
                }
                bidi_generate_content_client_message::MessageType::ClientContent(v) => {
                    struct_ser.serialize_field("clientContent", v)?;
                }
                bidi_generate_content_client_message::MessageType::RealtimeInput(v) => {
                    struct_ser.serialize_field("realtimeInput", v)?;
                }
                bidi_generate_content_client_message::MessageType::ToolResponse(v) => {
                    struct_ser.serialize_field("toolResponse", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentClientMessage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "setup",
            "client_content",
            "clientContent",
            "realtime_input",
            "realtimeInput",
            "tool_response",
            "toolResponse",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Setup,
            ClientContent,
            RealtimeInput,
            ToolResponse,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "setup" => Ok(GeneratedField::Setup),
                            "clientContent" | "client_content" => Ok(GeneratedField::ClientContent),
                            "realtimeInput" | "realtime_input" => Ok(GeneratedField::RealtimeInput),
                            "toolResponse" | "tool_response" => Ok(GeneratedField::ToolResponse),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentClientMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentClientMessage")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentClientMessage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut message_type__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Setup => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("setup"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_client_message::MessageType::Setup)
;
                        }
                        GeneratedField::ClientContent => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clientContent"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_client_message::MessageType::ClientContent)
;
                        }
                        GeneratedField::RealtimeInput => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("realtimeInput"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_client_message::MessageType::RealtimeInput)
;
                        }
                        GeneratedField::ToolResponse => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolResponse"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_client_message::MessageType::ToolResponse)
;
                        }
                    }
                }
                Ok(BidiGenerateContentClientMessage {
                    message_type: message_type__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentClientMessage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentRealtimeInput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.media_chunks.is_empty() {
            len += 1;
        }
        if self.audio.is_some() {
            len += 1;
        }
        if self.audio_stream_end.is_some() {
            len += 1;
        }
        if self.video.is_some() {
            len += 1;
        }
        if self.text.is_some() {
            len += 1;
        }
        if self.activity_start.is_some() {
            len += 1;
        }
        if self.activity_end.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput", len)?;
        if !self.media_chunks.is_empty() {
            struct_ser.serialize_field("mediaChunks", &self.media_chunks)?;
        }
        if let Some(v) = self.audio.as_ref() {
            struct_ser.serialize_field("audio", v)?;
        }
        if let Some(v) = self.audio_stream_end.as_ref() {
            struct_ser.serialize_field("audioStreamEnd", v)?;
        }
        if let Some(v) = self.video.as_ref() {
            struct_ser.serialize_field("video", v)?;
        }
        if let Some(v) = self.text.as_ref() {
            struct_ser.serialize_field("text", v)?;
        }
        if let Some(v) = self.activity_start.as_ref() {
            struct_ser.serialize_field("activityStart", v)?;
        }
        if let Some(v) = self.activity_end.as_ref() {
            struct_ser.serialize_field("activityEnd", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentRealtimeInput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "media_chunks",
            "mediaChunks",
            "audio",
            "audio_stream_end",
            "audioStreamEnd",
            "video",
            "text",
            "activity_start",
            "activityStart",
            "activity_end",
            "activityEnd",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MediaChunks,
            Audio,
            AudioStreamEnd,
            Video,
            Text,
            ActivityStart,
            ActivityEnd,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mediaChunks" | "media_chunks" => Ok(GeneratedField::MediaChunks),
                            "audio" => Ok(GeneratedField::Audio),
                            "audioStreamEnd" | "audio_stream_end" => Ok(GeneratedField::AudioStreamEnd),
                            "video" => Ok(GeneratedField::Video),
                            "text" => Ok(GeneratedField::Text),
                            "activityStart" | "activity_start" => Ok(GeneratedField::ActivityStart),
                            "activityEnd" | "activity_end" => Ok(GeneratedField::ActivityEnd),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentRealtimeInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentRealtimeInput, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut media_chunks__ = None;
                let mut audio__ = None;
                let mut audio_stream_end__ = None;
                let mut video__ = None;
                let mut text__ = None;
                let mut activity_start__ = None;
                let mut activity_end__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MediaChunks => {
                            if media_chunks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mediaChunks"));
                            }
                            media_chunks__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Audio => {
                            if audio__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audio"));
                            }
                            audio__ = map_.next_value()?;
                        }
                        GeneratedField::AudioStreamEnd => {
                            if audio_stream_end__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audioStreamEnd"));
                            }
                            audio_stream_end__ = map_.next_value()?;
                        }
                        GeneratedField::Video => {
                            if video__.is_some() {
                                return Err(serde::de::Error::duplicate_field("video"));
                            }
                            video__ = map_.next_value()?;
                        }
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = map_.next_value()?;
                        }
                        GeneratedField::ActivityStart => {
                            if activity_start__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activityStart"));
                            }
                            activity_start__ = map_.next_value()?;
                        }
                        GeneratedField::ActivityEnd => {
                            if activity_end__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activityEnd"));
                            }
                            activity_end__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BidiGenerateContentRealtimeInput {
                    media_chunks: media_chunks__.unwrap_or_default(),
                    audio: audio__,
                    audio_stream_end: audio_stream_end__,
                    video: video__,
                    text: text__,
                    activity_start: activity_start__,
                    activity_end: activity_end__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for bidi_generate_content_realtime_input::ActivityEnd {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityEnd", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for bidi_generate_content_realtime_input::ActivityEnd {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = bidi_generate_content_realtime_input::ActivityEnd;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityEnd")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<bidi_generate_content_realtime_input::ActivityEnd, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(bidi_generate_content_realtime_input::ActivityEnd {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityEnd", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for bidi_generate_content_realtime_input::ActivityStart {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityStart", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for bidi_generate_content_realtime_input::ActivityStart {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = bidi_generate_content_realtime_input::ActivityStart;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityStart")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<bidi_generate_content_realtime_input::ActivityStart, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(bidi_generate_content_realtime_input::ActivityStart {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentRealtimeInput.ActivityStart", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentServerContent {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.model_turn.is_some() {
            len += 1;
        }
        if self.generation_complete {
            len += 1;
        }
        if self.turn_complete {
            len += 1;
        }
        if self.interrupted {
            len += 1;
        }
        if self.grounding_metadata.is_some() {
            len += 1;
        }
        if self.input_transcription.is_some() {
            len += 1;
        }
        if self.output_transcription.is_some() {
            len += 1;
        }
        if self.url_context_metadata.is_some() {
            len += 1;
        }
        if self.waiting_for_input {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentServerContent", len)?;
        if let Some(v) = self.model_turn.as_ref() {
            struct_ser.serialize_field("modelTurn", v)?;
        }
        if self.generation_complete {
            struct_ser.serialize_field("generationComplete", &self.generation_complete)?;
        }
        if self.turn_complete {
            struct_ser.serialize_field("turnComplete", &self.turn_complete)?;
        }
        if self.interrupted {
            struct_ser.serialize_field("interrupted", &self.interrupted)?;
        }
        if let Some(v) = self.grounding_metadata.as_ref() {
            struct_ser.serialize_field("groundingMetadata", v)?;
        }
        if let Some(v) = self.input_transcription.as_ref() {
            struct_ser.serialize_field("inputTranscription", v)?;
        }
        if let Some(v) = self.output_transcription.as_ref() {
            struct_ser.serialize_field("outputTranscription", v)?;
        }
        if let Some(v) = self.url_context_metadata.as_ref() {
            struct_ser.serialize_field("urlContextMetadata", v)?;
        }
        if self.waiting_for_input {
            struct_ser.serialize_field("waitingForInput", &self.waiting_for_input)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentServerContent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model_turn",
            "modelTurn",
            "generation_complete",
            "generationComplete",
            "turn_complete",
            "turnComplete",
            "interrupted",
            "grounding_metadata",
            "groundingMetadata",
            "input_transcription",
            "inputTranscription",
            "output_transcription",
            "outputTranscription",
            "url_context_metadata",
            "urlContextMetadata",
            "waiting_for_input",
            "waitingForInput",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ModelTurn,
            GenerationComplete,
            TurnComplete,
            Interrupted,
            GroundingMetadata,
            InputTranscription,
            OutputTranscription,
            UrlContextMetadata,
            WaitingForInput,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "modelTurn" | "model_turn" => Ok(GeneratedField::ModelTurn),
                            "generationComplete" | "generation_complete" => Ok(GeneratedField::GenerationComplete),
                            "turnComplete" | "turn_complete" => Ok(GeneratedField::TurnComplete),
                            "interrupted" => Ok(GeneratedField::Interrupted),
                            "groundingMetadata" | "grounding_metadata" => Ok(GeneratedField::GroundingMetadata),
                            "inputTranscription" | "input_transcription" => Ok(GeneratedField::InputTranscription),
                            "outputTranscription" | "output_transcription" => Ok(GeneratedField::OutputTranscription),
                            "urlContextMetadata" | "url_context_metadata" => Ok(GeneratedField::UrlContextMetadata),
                            "waitingForInput" | "waiting_for_input" => Ok(GeneratedField::WaitingForInput),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentServerContent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentServerContent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentServerContent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model_turn__ = None;
                let mut generation_complete__ = None;
                let mut turn_complete__ = None;
                let mut interrupted__ = None;
                let mut grounding_metadata__ = None;
                let mut input_transcription__ = None;
                let mut output_transcription__ = None;
                let mut url_context_metadata__ = None;
                let mut waiting_for_input__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ModelTurn => {
                            if model_turn__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelTurn"));
                            }
                            model_turn__ = map_.next_value()?;
                        }
                        GeneratedField::GenerationComplete => {
                            if generation_complete__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generationComplete"));
                            }
                            generation_complete__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TurnComplete => {
                            if turn_complete__.is_some() {
                                return Err(serde::de::Error::duplicate_field("turnComplete"));
                            }
                            turn_complete__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Interrupted => {
                            if interrupted__.is_some() {
                                return Err(serde::de::Error::duplicate_field("interrupted"));
                            }
                            interrupted__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GroundingMetadata => {
                            if grounding_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingMetadata"));
                            }
                            grounding_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::InputTranscription => {
                            if input_transcription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inputTranscription"));
                            }
                            input_transcription__ = map_.next_value()?;
                        }
                        GeneratedField::OutputTranscription => {
                            if output_transcription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputTranscription"));
                            }
                            output_transcription__ = map_.next_value()?;
                        }
                        GeneratedField::UrlContextMetadata => {
                            if url_context_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("urlContextMetadata"));
                            }
                            url_context_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::WaitingForInput => {
                            if waiting_for_input__.is_some() {
                                return Err(serde::de::Error::duplicate_field("waitingForInput"));
                            }
                            waiting_for_input__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentServerContent {
                    model_turn: model_turn__,
                    generation_complete: generation_complete__.unwrap_or_default(),
                    turn_complete: turn_complete__.unwrap_or_default(),
                    interrupted: interrupted__.unwrap_or_default(),
                    grounding_metadata: grounding_metadata__,
                    input_transcription: input_transcription__,
                    output_transcription: output_transcription__,
                    url_context_metadata: url_context_metadata__,
                    waiting_for_input: waiting_for_input__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentServerContent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentServerMessage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.usage_metadata.is_some() {
            len += 1;
        }
        if self.message_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentServerMessage", len)?;
        if let Some(v) = self.usage_metadata.as_ref() {
            struct_ser.serialize_field("usageMetadata", v)?;
        }
        if let Some(v) = self.message_type.as_ref() {
            match v {
                bidi_generate_content_server_message::MessageType::SetupComplete(v) => {
                    struct_ser.serialize_field("setupComplete", v)?;
                }
                bidi_generate_content_server_message::MessageType::ServerContent(v) => {
                    struct_ser.serialize_field("serverContent", v)?;
                }
                bidi_generate_content_server_message::MessageType::ToolCall(v) => {
                    struct_ser.serialize_field("toolCall", v)?;
                }
                bidi_generate_content_server_message::MessageType::ToolCallCancellation(v) => {
                    struct_ser.serialize_field("toolCallCancellation", v)?;
                }
                bidi_generate_content_server_message::MessageType::GoAway(v) => {
                    struct_ser.serialize_field("goAway", v)?;
                }
                bidi_generate_content_server_message::MessageType::SessionResumptionUpdate(v) => {
                    struct_ser.serialize_field("sessionResumptionUpdate", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentServerMessage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "usage_metadata",
            "usageMetadata",
            "setup_complete",
            "setupComplete",
            "server_content",
            "serverContent",
            "tool_call",
            "toolCall",
            "tool_call_cancellation",
            "toolCallCancellation",
            "go_away",
            "goAway",
            "session_resumption_update",
            "sessionResumptionUpdate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            UsageMetadata,
            SetupComplete,
            ServerContent,
            ToolCall,
            ToolCallCancellation,
            GoAway,
            SessionResumptionUpdate,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "usageMetadata" | "usage_metadata" => Ok(GeneratedField::UsageMetadata),
                            "setupComplete" | "setup_complete" => Ok(GeneratedField::SetupComplete),
                            "serverContent" | "server_content" => Ok(GeneratedField::ServerContent),
                            "toolCall" | "tool_call" => Ok(GeneratedField::ToolCall),
                            "toolCallCancellation" | "tool_call_cancellation" => Ok(GeneratedField::ToolCallCancellation),
                            "goAway" | "go_away" => Ok(GeneratedField::GoAway),
                            "sessionResumptionUpdate" | "session_resumption_update" => Ok(GeneratedField::SessionResumptionUpdate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentServerMessage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentServerMessage")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentServerMessage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut usage_metadata__ = None;
                let mut message_type__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::UsageMetadata => {
                            if usage_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("usageMetadata"));
                            }
                            usage_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::SetupComplete => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("setupComplete"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::SetupComplete)
;
                        }
                        GeneratedField::ServerContent => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("serverContent"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::ServerContent)
;
                        }
                        GeneratedField::ToolCall => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolCall"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::ToolCall)
;
                        }
                        GeneratedField::ToolCallCancellation => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolCallCancellation"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::ToolCallCancellation)
;
                        }
                        GeneratedField::GoAway => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("goAway"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::GoAway)
;
                        }
                        GeneratedField::SessionResumptionUpdate => {
                            if message_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionResumptionUpdate"));
                            }
                            message_type__ = map_.next_value::<::std::option::Option<_>>()?.map(bidi_generate_content_server_message::MessageType::SessionResumptionUpdate)
;
                        }
                    }
                }
                Ok(BidiGenerateContentServerMessage {
                    usage_metadata: usage_metadata__,
                    message_type: message_type__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentServerMessage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentSetup {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if self.generation_config.is_some() {
            len += 1;
        }
        if self.system_instruction.is_some() {
            len += 1;
        }
        if !self.tools.is_empty() {
            len += 1;
        }
        if self.realtime_input_config.is_some() {
            len += 1;
        }
        if self.session_resumption.is_some() {
            len += 1;
        }
        if self.context_window_compression.is_some() {
            len += 1;
        }
        if self.input_audio_transcription.is_some() {
            len += 1;
        }
        if self.output_audio_transcription.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentSetup", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if let Some(v) = self.generation_config.as_ref() {
            struct_ser.serialize_field("generationConfig", v)?;
        }
        if let Some(v) = self.system_instruction.as_ref() {
            struct_ser.serialize_field("systemInstruction", v)?;
        }
        if !self.tools.is_empty() {
            struct_ser.serialize_field("tools", &self.tools)?;
        }
        if let Some(v) = self.realtime_input_config.as_ref() {
            struct_ser.serialize_field("realtimeInputConfig", v)?;
        }
        if let Some(v) = self.session_resumption.as_ref() {
            struct_ser.serialize_field("sessionResumption", v)?;
        }
        if let Some(v) = self.context_window_compression.as_ref() {
            struct_ser.serialize_field("contextWindowCompression", v)?;
        }
        if let Some(v) = self.input_audio_transcription.as_ref() {
            struct_ser.serialize_field("inputAudioTranscription", v)?;
        }
        if let Some(v) = self.output_audio_transcription.as_ref() {
            struct_ser.serialize_field("outputAudioTranscription", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentSetup {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "generation_config",
            "generationConfig",
            "system_instruction",
            "systemInstruction",
            "tools",
            "realtime_input_config",
            "realtimeInputConfig",
            "session_resumption",
            "sessionResumption",
            "context_window_compression",
            "contextWindowCompression",
            "input_audio_transcription",
            "inputAudioTranscription",
            "output_audio_transcription",
            "outputAudioTranscription",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            GenerationConfig,
            SystemInstruction,
            Tools,
            RealtimeInputConfig,
            SessionResumption,
            ContextWindowCompression,
            InputAudioTranscription,
            OutputAudioTranscription,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "generationConfig" | "generation_config" => Ok(GeneratedField::GenerationConfig),
                            "systemInstruction" | "system_instruction" => Ok(GeneratedField::SystemInstruction),
                            "tools" => Ok(GeneratedField::Tools),
                            "realtimeInputConfig" | "realtime_input_config" => Ok(GeneratedField::RealtimeInputConfig),
                            "sessionResumption" | "session_resumption" => Ok(GeneratedField::SessionResumption),
                            "contextWindowCompression" | "context_window_compression" => Ok(GeneratedField::ContextWindowCompression),
                            "inputAudioTranscription" | "input_audio_transcription" => Ok(GeneratedField::InputAudioTranscription),
                            "outputAudioTranscription" | "output_audio_transcription" => Ok(GeneratedField::OutputAudioTranscription),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentSetup;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentSetup")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentSetup, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut generation_config__ = None;
                let mut system_instruction__ = None;
                let mut tools__ = None;
                let mut realtime_input_config__ = None;
                let mut session_resumption__ = None;
                let mut context_window_compression__ = None;
                let mut input_audio_transcription__ = None;
                let mut output_audio_transcription__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GenerationConfig => {
                            if generation_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generationConfig"));
                            }
                            generation_config__ = map_.next_value()?;
                        }
                        GeneratedField::SystemInstruction => {
                            if system_instruction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("systemInstruction"));
                            }
                            system_instruction__ = map_.next_value()?;
                        }
                        GeneratedField::Tools => {
                            if tools__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tools"));
                            }
                            tools__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RealtimeInputConfig => {
                            if realtime_input_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("realtimeInputConfig"));
                            }
                            realtime_input_config__ = map_.next_value()?;
                        }
                        GeneratedField::SessionResumption => {
                            if session_resumption__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionResumption"));
                            }
                            session_resumption__ = map_.next_value()?;
                        }
                        GeneratedField::ContextWindowCompression => {
                            if context_window_compression__.is_some() {
                                return Err(serde::de::Error::duplicate_field("contextWindowCompression"));
                            }
                            context_window_compression__ = map_.next_value()?;
                        }
                        GeneratedField::InputAudioTranscription => {
                            if input_audio_transcription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inputAudioTranscription"));
                            }
                            input_audio_transcription__ = map_.next_value()?;
                        }
                        GeneratedField::OutputAudioTranscription => {
                            if output_audio_transcription__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputAudioTranscription"));
                            }
                            output_audio_transcription__ = map_.next_value()?;
                        }
                    }
                }
                Ok(BidiGenerateContentSetup {
                    model: model__.unwrap_or_default(),
                    generation_config: generation_config__,
                    system_instruction: system_instruction__,
                    tools: tools__.unwrap_or_default(),
                    realtime_input_config: realtime_input_config__,
                    session_resumption: session_resumption__,
                    context_window_compression: context_window_compression__,
                    input_audio_transcription: input_audio_transcription__,
                    output_audio_transcription: output_audio_transcription__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentSetup", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentSetupComplete {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentSetupComplete", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentSetupComplete {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentSetupComplete;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentSetupComplete")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentSetupComplete, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(BidiGenerateContentSetupComplete {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentSetupComplete", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentToolCall {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.function_calls.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolCall", len)?;
        if !self.function_calls.is_empty() {
            struct_ser.serialize_field("functionCalls", &self.function_calls)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentToolCall {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function_calls",
            "functionCalls",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FunctionCalls,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "functionCalls" | "function_calls" => Ok(GeneratedField::FunctionCalls),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentToolCall;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentToolCall")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentToolCall, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function_calls__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::FunctionCalls => {
                            if function_calls__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionCalls"));
                            }
                            function_calls__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentToolCall {
                    function_calls: function_calls__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolCall", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentToolCallCancellation {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.ids.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolCallCancellation", len)?;
        if !self.ids.is_empty() {
            struct_ser.serialize_field("ids", &self.ids)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentToolCallCancellation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ids",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Ids,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "ids" => Ok(GeneratedField::Ids),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentToolCallCancellation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentToolCallCancellation")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentToolCallCancellation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut ids__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Ids => {
                            if ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ids"));
                            }
                            ids__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentToolCallCancellation {
                    ids: ids__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolCallCancellation", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentToolResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.function_responses.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolResponse", len)?;
        if !self.function_responses.is_empty() {
            struct_ser.serialize_field("functionResponses", &self.function_responses)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentToolResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function_responses",
            "functionResponses",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FunctionResponses,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "functionResponses" | "function_responses" => Ok(GeneratedField::FunctionResponses),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentToolResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentToolResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentToolResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function_responses__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::FunctionResponses => {
                            if function_responses__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionResponses"));
                            }
                            function_responses__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentToolResponse {
                    function_responses: function_responses__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentToolResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for BidiGenerateContentTranscription {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.text.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentTranscription", len)?;
        if !self.text.is_empty() {
            struct_ser.serialize_field("text", &self.text)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for BidiGenerateContentTranscription {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "text",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Text,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "text" => Ok(GeneratedField::Text),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = BidiGenerateContentTranscription;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.BidiGenerateContentTranscription")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<BidiGenerateContentTranscription, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut text__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(BidiGenerateContentTranscription {
                    text: text__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.BidiGenerateContentTranscription", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Blob {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.mime_type.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Blob", len)?;
        if !self.mime_type.is_empty() {
            struct_ser.serialize_field("mimeType", &self.mime_type)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Blob {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mime_type",
            "mimeType",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MimeType,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mimeType" | "mime_type" => Ok(GeneratedField::MimeType),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Blob;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Blob")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Blob, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mime_type__ = None;
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MimeType => {
                            if mime_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mimeType"));
                            }
                            mime_type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Blob {
                    mime_type: mime_type__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Blob", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Candidate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.index.is_some() {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        if self.finish_reason != 0 {
            len += 1;
        }
        if self.finish_message.is_some() {
            len += 1;
        }
        if !self.safety_ratings.is_empty() {
            len += 1;
        }
        if self.citation_metadata.is_some() {
            len += 1;
        }
        if self.token_count != 0 {
            len += 1;
        }
        if !self.grounding_attributions.is_empty() {
            len += 1;
        }
        if self.grounding_metadata.is_some() {
            len += 1;
        }
        if self.avg_logprobs != 0. {
            len += 1;
        }
        if self.logprobs_result.is_some() {
            len += 1;
        }
        if self.url_context_metadata.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Candidate", len)?;
        if let Some(v) = self.index.as_ref() {
            struct_ser.serialize_field("index", v)?;
        }
        if let Some(v) = self.content.as_ref() {
            struct_ser.serialize_field("content", v)?;
        }
        if self.finish_reason != 0 {
            let v = candidate::FinishReason::try_from(self.finish_reason)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.finish_reason)))?;
            struct_ser.serialize_field("finishReason", &v)?;
        }
        if let Some(v) = self.finish_message.as_ref() {
            struct_ser.serialize_field("finishMessage", v)?;
        }
        if !self.safety_ratings.is_empty() {
            struct_ser.serialize_field("safetyRatings", &self.safety_ratings)?;
        }
        if let Some(v) = self.citation_metadata.as_ref() {
            struct_ser.serialize_field("citationMetadata", v)?;
        }
        if self.token_count != 0 {
            struct_ser.serialize_field("tokenCount", &self.token_count)?;
        }
        if !self.grounding_attributions.is_empty() {
            struct_ser.serialize_field("groundingAttributions", &self.grounding_attributions)?;
        }
        if let Some(v) = self.grounding_metadata.as_ref() {
            struct_ser.serialize_field("groundingMetadata", v)?;
        }
        if self.avg_logprobs != 0. {
            struct_ser.serialize_field("avgLogprobs", &self.avg_logprobs)?;
        }
        if let Some(v) = self.logprobs_result.as_ref() {
            struct_ser.serialize_field("logprobsResult", v)?;
        }
        if let Some(v) = self.url_context_metadata.as_ref() {
            struct_ser.serialize_field("urlContextMetadata", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Candidate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "index",
            "content",
            "finish_reason",
            "finishReason",
            "finish_message",
            "finishMessage",
            "safety_ratings",
            "safetyRatings",
            "citation_metadata",
            "citationMetadata",
            "token_count",
            "tokenCount",
            "grounding_attributions",
            "groundingAttributions",
            "grounding_metadata",
            "groundingMetadata",
            "avg_logprobs",
            "avgLogprobs",
            "logprobs_result",
            "logprobsResult",
            "url_context_metadata",
            "urlContextMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Index,
            Content,
            FinishReason,
            FinishMessage,
            SafetyRatings,
            CitationMetadata,
            TokenCount,
            GroundingAttributions,
            GroundingMetadata,
            AvgLogprobs,
            LogprobsResult,
            UrlContextMetadata,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "index" => Ok(GeneratedField::Index),
                            "content" => Ok(GeneratedField::Content),
                            "finishReason" | "finish_reason" => Ok(GeneratedField::FinishReason),
                            "finishMessage" | "finish_message" => Ok(GeneratedField::FinishMessage),
                            "safetyRatings" | "safety_ratings" => Ok(GeneratedField::SafetyRatings),
                            "citationMetadata" | "citation_metadata" => Ok(GeneratedField::CitationMetadata),
                            "tokenCount" | "token_count" => Ok(GeneratedField::TokenCount),
                            "groundingAttributions" | "grounding_attributions" => Ok(GeneratedField::GroundingAttributions),
                            "groundingMetadata" | "grounding_metadata" => Ok(GeneratedField::GroundingMetadata),
                            "avgLogprobs" | "avg_logprobs" => Ok(GeneratedField::AvgLogprobs),
                            "logprobsResult" | "logprobs_result" => Ok(GeneratedField::LogprobsResult),
                            "urlContextMetadata" | "url_context_metadata" => Ok(GeneratedField::UrlContextMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Candidate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Candidate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Candidate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut index__ = None;
                let mut content__ = None;
                let mut finish_reason__ = None;
                let mut finish_message__ = None;
                let mut safety_ratings__ = None;
                let mut citation_metadata__ = None;
                let mut token_count__ = None;
                let mut grounding_attributions__ = None;
                let mut grounding_metadata__ = None;
                let mut avg_logprobs__ = None;
                let mut logprobs_result__ = None;
                let mut url_context_metadata__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = map_.next_value()?;
                        }
                        GeneratedField::FinishReason => {
                            if finish_reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("finishReason"));
                            }
                            finish_reason__ = Some(map_.next_value::<candidate::FinishReason>()? as i32);
                        }
                        GeneratedField::FinishMessage => {
                            if finish_message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("finishMessage"));
                            }
                            finish_message__ = map_.next_value()?;
                        }
                        GeneratedField::SafetyRatings => {
                            if safety_ratings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("safetyRatings"));
                            }
                            safety_ratings__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CitationMetadata => {
                            if citation_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("citationMetadata"));
                            }
                            citation_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::TokenCount => {
                            if token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenCount"));
                            }
                            token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::GroundingAttributions => {
                            if grounding_attributions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingAttributions"));
                            }
                            grounding_attributions__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GroundingMetadata => {
                            if grounding_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingMetadata"));
                            }
                            grounding_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::AvgLogprobs => {
                            if avg_logprobs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("avgLogprobs"));
                            }
                            avg_logprobs__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LogprobsResult => {
                            if logprobs_result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logprobsResult"));
                            }
                            logprobs_result__ = map_.next_value()?;
                        }
                        GeneratedField::UrlContextMetadata => {
                            if url_context_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("urlContextMetadata"));
                            }
                            url_context_metadata__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Candidate {
                    index: index__,
                    content: content__,
                    finish_reason: finish_reason__.unwrap_or_default(),
                    finish_message: finish_message__,
                    safety_ratings: safety_ratings__.unwrap_or_default(),
                    citation_metadata: citation_metadata__,
                    token_count: token_count__.unwrap_or_default(),
                    grounding_attributions: grounding_attributions__.unwrap_or_default(),
                    grounding_metadata: grounding_metadata__,
                    avg_logprobs: avg_logprobs__.unwrap_or_default(),
                    logprobs_result: logprobs_result__,
                    url_context_metadata: url_context_metadata__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Candidate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for candidate::FinishReason {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "FINISH_REASON_UNSPECIFIED",
            Self::Stop => "STOP",
            Self::MaxTokens => "MAX_TOKENS",
            Self::Safety => "SAFETY",
            Self::Recitation => "RECITATION",
            Self::Language => "LANGUAGE",
            Self::Other => "OTHER",
            Self::Blocklist => "BLOCKLIST",
            Self::ProhibitedContent => "PROHIBITED_CONTENT",
            Self::Spii => "SPII",
            Self::MalformedFunctionCall => "MALFORMED_FUNCTION_CALL",
            Self::ImageSafety => "IMAGE_SAFETY",
            Self::ImageProhibitedContent => "IMAGE_PROHIBITED_CONTENT",
            Self::ImageOther => "IMAGE_OTHER",
            Self::NoImage => "NO_IMAGE",
            Self::ImageRecitation => "IMAGE_RECITATION",
            Self::UnexpectedToolCall => "UNEXPECTED_TOOL_CALL",
            Self::TooManyToolCalls => "TOO_MANY_TOOL_CALLS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for candidate::FinishReason {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "FINISH_REASON_UNSPECIFIED",
            "STOP",
            "MAX_TOKENS",
            "SAFETY",
            "RECITATION",
            "LANGUAGE",
            "OTHER",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
            "MALFORMED_FUNCTION_CALL",
            "IMAGE_SAFETY",
            "IMAGE_PROHIBITED_CONTENT",
            "IMAGE_OTHER",
            "NO_IMAGE",
            "IMAGE_RECITATION",
            "UNEXPECTED_TOOL_CALL",
            "TOO_MANY_TOOL_CALLS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = candidate::FinishReason;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "FINISH_REASON_UNSPECIFIED" => Ok(candidate::FinishReason::Unspecified),
                    "STOP" => Ok(candidate::FinishReason::Stop),
                    "MAX_TOKENS" => Ok(candidate::FinishReason::MaxTokens),
                    "SAFETY" => Ok(candidate::FinishReason::Safety),
                    "RECITATION" => Ok(candidate::FinishReason::Recitation),
                    "LANGUAGE" => Ok(candidate::FinishReason::Language),
                    "OTHER" => Ok(candidate::FinishReason::Other),
                    "BLOCKLIST" => Ok(candidate::FinishReason::Blocklist),
                    "PROHIBITED_CONTENT" => Ok(candidate::FinishReason::ProhibitedContent),
                    "SPII" => Ok(candidate::FinishReason::Spii),
                    "MALFORMED_FUNCTION_CALL" => Ok(candidate::FinishReason::MalformedFunctionCall),
                    "IMAGE_SAFETY" => Ok(candidate::FinishReason::ImageSafety),
                    "IMAGE_PROHIBITED_CONTENT" => Ok(candidate::FinishReason::ImageProhibitedContent),
                    "IMAGE_OTHER" => Ok(candidate::FinishReason::ImageOther),
                    "NO_IMAGE" => Ok(candidate::FinishReason::NoImage),
                    "IMAGE_RECITATION" => Ok(candidate::FinishReason::ImageRecitation),
                    "UNEXPECTED_TOOL_CALL" => Ok(candidate::FinishReason::UnexpectedToolCall),
                    "TOO_MANY_TOOL_CALLS" => Ok(candidate::FinishReason::TooManyToolCalls),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Chunk {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        if !self.custom_metadata.is_empty() {
            len += 1;
        }
        if self.create_time.is_some() {
            len += 1;
        }
        if self.update_time.is_some() {
            len += 1;
        }
        if self.state != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Chunk", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if let Some(v) = self.data.as_ref() {
            struct_ser.serialize_field("data", v)?;
        }
        if !self.custom_metadata.is_empty() {
            struct_ser.serialize_field("customMetadata", &self.custom_metadata)?;
        }
        if let Some(v) = self.create_time.as_ref() {
            struct_ser.serialize_field("createTime", v)?;
        }
        if let Some(v) = self.update_time.as_ref() {
            struct_ser.serialize_field("updateTime", v)?;
        }
        if self.state != 0 {
            let v = chunk::State::try_from(self.state)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.state)))?;
            struct_ser.serialize_field("state", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Chunk {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "data",
            "custom_metadata",
            "customMetadata",
            "create_time",
            "createTime",
            "update_time",
            "updateTime",
            "state",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Data,
            CustomMetadata,
            CreateTime,
            UpdateTime,
            State,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "data" => Ok(GeneratedField::Data),
                            "customMetadata" | "custom_metadata" => Ok(GeneratedField::CustomMetadata),
                            "createTime" | "create_time" => Ok(GeneratedField::CreateTime),
                            "updateTime" | "update_time" => Ok(GeneratedField::UpdateTime),
                            "state" => Ok(GeneratedField::State),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Chunk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Chunk")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Chunk, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut data__ = None;
                let mut custom_metadata__ = None;
                let mut create_time__ = None;
                let mut update_time__ = None;
                let mut state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = map_.next_value()?;
                        }
                        GeneratedField::CustomMetadata => {
                            if custom_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("customMetadata"));
                            }
                            custom_metadata__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CreateTime => {
                            if create_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createTime"));
                            }
                            create_time__ = map_.next_value()?;
                        }
                        GeneratedField::UpdateTime => {
                            if update_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateTime"));
                            }
                            update_time__ = map_.next_value()?;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = Some(map_.next_value::<chunk::State>()? as i32);
                        }
                    }
                }
                Ok(Chunk {
                    name: name__.unwrap_or_default(),
                    data: data__,
                    custom_metadata: custom_metadata__.unwrap_or_default(),
                    create_time: create_time__,
                    update_time: update_time__,
                    state: state__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Chunk", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for chunk::State {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "STATE_UNSPECIFIED",
            Self::PendingProcessing => "STATE_PENDING_PROCESSING",
            Self::Active => "STATE_ACTIVE",
            Self::Failed => "STATE_FAILED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for chunk::State {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "STATE_UNSPECIFIED",
            "STATE_PENDING_PROCESSING",
            "STATE_ACTIVE",
            "STATE_FAILED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = chunk::State;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "STATE_UNSPECIFIED" => Ok(chunk::State::Unspecified),
                    "STATE_PENDING_PROCESSING" => Ok(chunk::State::PendingProcessing),
                    "STATE_ACTIVE" => Ok(chunk::State::Active),
                    "STATE_FAILED" => Ok(chunk::State::Failed),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ChunkData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ChunkData", len)?;
        if let Some(v) = self.data.as_ref() {
            match v {
                chunk_data::Data::StringValue(v) => {
                    struct_ser.serialize_field("stringValue", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ChunkData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "string_value",
            "stringValue",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StringValue,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "stringValue" | "string_value" => Ok(GeneratedField::StringValue),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ChunkData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ChunkData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ChunkData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StringValue => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stringValue"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(chunk_data::Data::StringValue);
                        }
                    }
                }
                Ok(ChunkData {
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ChunkData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CitationMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.citation_sources.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CitationMetadata", len)?;
        if !self.citation_sources.is_empty() {
            struct_ser.serialize_field("citationSources", &self.citation_sources)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CitationMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "citation_sources",
            "citationSources",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CitationSources,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "citationSources" | "citation_sources" => Ok(GeneratedField::CitationSources),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CitationMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CitationMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CitationMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut citation_sources__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CitationSources => {
                            if citation_sources__.is_some() {
                                return Err(serde::de::Error::duplicate_field("citationSources"));
                            }
                            citation_sources__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CitationMetadata {
                    citation_sources: citation_sources__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CitationMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CitationSource {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_index.is_some() {
            len += 1;
        }
        if self.end_index.is_some() {
            len += 1;
        }
        if self.uri.is_some() {
            len += 1;
        }
        if self.license.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CitationSource", len)?;
        if let Some(v) = self.start_index.as_ref() {
            struct_ser.serialize_field("startIndex", v)?;
        }
        if let Some(v) = self.end_index.as_ref() {
            struct_ser.serialize_field("endIndex", v)?;
        }
        if let Some(v) = self.uri.as_ref() {
            struct_ser.serialize_field("uri", v)?;
        }
        if let Some(v) = self.license.as_ref() {
            struct_ser.serialize_field("license", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CitationSource {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_index",
            "startIndex",
            "end_index",
            "endIndex",
            "uri",
            "license",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartIndex,
            EndIndex,
            Uri,
            License,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "startIndex" | "start_index" => Ok(GeneratedField::StartIndex),
                            "endIndex" | "end_index" => Ok(GeneratedField::EndIndex),
                            "uri" => Ok(GeneratedField::Uri),
                            "license" => Ok(GeneratedField::License),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CitationSource;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CitationSource")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CitationSource, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_index__ = None;
                let mut end_index__ = None;
                let mut uri__ = None;
                let mut license__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartIndex => {
                            if start_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startIndex"));
                            }
                            start_index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::EndIndex => {
                            if end_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endIndex"));
                            }
                            end_index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Uri => {
                            if uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uri"));
                            }
                            uri__ = map_.next_value()?;
                        }
                        GeneratedField::License => {
                            if license__.is_some() {
                                return Err(serde::de::Error::duplicate_field("license"));
                            }
                            license__ = map_.next_value()?;
                        }
                    }
                }
                Ok(CitationSource {
                    start_index: start_index__,
                    end_index: end_index__,
                    uri: uri__,
                    license: license__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CitationSource", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CodeExecution {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CodeExecution", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CodeExecution {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CodeExecution;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CodeExecution")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CodeExecution, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(CodeExecution {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CodeExecution", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CodeExecutionResult {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.outcome != 0 {
            len += 1;
        }
        if !self.output.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CodeExecutionResult", len)?;
        if self.outcome != 0 {
            let v = code_execution_result::Outcome::try_from(self.outcome)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.outcome)))?;
            struct_ser.serialize_field("outcome", &v)?;
        }
        if !self.output.is_empty() {
            struct_ser.serialize_field("output", &self.output)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CodeExecutionResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "outcome",
            "output",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Outcome,
            Output,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "outcome" => Ok(GeneratedField::Outcome),
                            "output" => Ok(GeneratedField::Output),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CodeExecutionResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CodeExecutionResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CodeExecutionResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut outcome__ = None;
                let mut output__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Outcome => {
                            if outcome__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outcome"));
                            }
                            outcome__ = Some(map_.next_value::<code_execution_result::Outcome>()? as i32);
                        }
                        GeneratedField::Output => {
                            if output__.is_some() {
                                return Err(serde::de::Error::duplicate_field("output"));
                            }
                            output__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CodeExecutionResult {
                    outcome: outcome__.unwrap_or_default(),
                    output: output__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CodeExecutionResult", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for code_execution_result::Outcome {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "OUTCOME_UNSPECIFIED",
            Self::Ok => "OUTCOME_OK",
            Self::Failed => "OUTCOME_FAILED",
            Self::DeadlineExceeded => "OUTCOME_DEADLINE_EXCEEDED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for code_execution_result::Outcome {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "OUTCOME_UNSPECIFIED",
            "OUTCOME_OK",
            "OUTCOME_FAILED",
            "OUTCOME_DEADLINE_EXCEEDED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = code_execution_result::Outcome;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "OUTCOME_UNSPECIFIED" => Ok(code_execution_result::Outcome::Unspecified),
                    "OUTCOME_OK" => Ok(code_execution_result::Outcome::Ok),
                    "OUTCOME_FAILED" => Ok(code_execution_result::Outcome::Failed),
                    "OUTCOME_DEADLINE_EXCEEDED" => Ok(code_execution_result::Outcome::DeadlineExceeded),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Condition {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.operation != 0 {
            len += 1;
        }
        if self.value.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Condition", len)?;
        if self.operation != 0 {
            let v = condition::Operator::try_from(self.operation)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.operation)))?;
            struct_ser.serialize_field("operation", &v)?;
        }
        if let Some(v) = self.value.as_ref() {
            match v {
                condition::Value::StringValue(v) => {
                    struct_ser.serialize_field("stringValue", v)?;
                }
                condition::Value::NumericValue(v) => {
                    struct_ser.serialize_field("numericValue", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Condition {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "operation",
            "string_value",
            "stringValue",
            "numeric_value",
            "numericValue",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Operation,
            StringValue,
            NumericValue,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "operation" => Ok(GeneratedField::Operation),
                            "stringValue" | "string_value" => Ok(GeneratedField::StringValue),
                            "numericValue" | "numeric_value" => Ok(GeneratedField::NumericValue),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Condition;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Condition")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Condition, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut operation__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Operation => {
                            if operation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("operation"));
                            }
                            operation__ = Some(map_.next_value::<condition::Operator>()? as i32);
                        }
                        GeneratedField::StringValue => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stringValue"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(condition::Value::StringValue);
                        }
                        GeneratedField::NumericValue => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numericValue"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| condition::Value::NumericValue(x.0));
                        }
                    }
                }
                Ok(Condition {
                    operation: operation__.unwrap_or_default(),
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Condition", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for condition::Operator {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "OPERATOR_UNSPECIFIED",
            Self::Less => "LESS",
            Self::LessEqual => "LESS_EQUAL",
            Self::Equal => "EQUAL",
            Self::GreaterEqual => "GREATER_EQUAL",
            Self::Greater => "GREATER",
            Self::NotEqual => "NOT_EQUAL",
            Self::Includes => "INCLUDES",
            Self::Excludes => "EXCLUDES",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for condition::Operator {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "OPERATOR_UNSPECIFIED",
            "LESS",
            "LESS_EQUAL",
            "EQUAL",
            "GREATER_EQUAL",
            "GREATER",
            "NOT_EQUAL",
            "INCLUDES",
            "EXCLUDES",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = condition::Operator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "OPERATOR_UNSPECIFIED" => Ok(condition::Operator::Unspecified),
                    "LESS" => Ok(condition::Operator::Less),
                    "LESS_EQUAL" => Ok(condition::Operator::LessEqual),
                    "EQUAL" => Ok(condition::Operator::Equal),
                    "GREATER_EQUAL" => Ok(condition::Operator::GreaterEqual),
                    "GREATER" => Ok(condition::Operator::Greater),
                    "NOT_EQUAL" => Ok(condition::Operator::NotEqual),
                    "INCLUDES" => Ok(condition::Operator::Includes),
                    "EXCLUDES" => Ok(condition::Operator::Excludes),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Content {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.parts.is_empty() {
            len += 1;
        }
        if !self.role.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Content", len)?;
        if !self.parts.is_empty() {
            struct_ser.serialize_field("parts", &self.parts)?;
        }
        if !self.role.is_empty() {
            struct_ser.serialize_field("role", &self.role)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Content {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "parts",
            "role",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Parts,
            Role,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "parts" => Ok(GeneratedField::Parts),
                            "role" => Ok(GeneratedField::Role),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Content;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Content")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Content, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut parts__ = None;
                let mut role__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Parts => {
                            if parts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parts"));
                            }
                            parts__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Role => {
                            if role__.is_some() {
                                return Err(serde::de::Error::duplicate_field("role"));
                            }
                            role__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Content {
                    parts: parts__.unwrap_or_default(),
                    role: role__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Content", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ContentEmbedding {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.values.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ContentEmbedding", len)?;
        if !self.values.is_empty() {
            struct_ser.serialize_field("values", &self.values)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ContentEmbedding {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "values",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Values,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "values" => Ok(GeneratedField::Values),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ContentEmbedding;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ContentEmbedding")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ContentEmbedding, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut values__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Values => {
                            if values__.is_some() {
                                return Err(serde::de::Error::duplicate_field("values"));
                            }
                            values__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(ContentEmbedding {
                    values: values__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ContentEmbedding", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ContentFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.reason != 0 {
            len += 1;
        }
        if self.message.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ContentFilter", len)?;
        if self.reason != 0 {
            let v = content_filter::BlockedReason::try_from(self.reason)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.reason)))?;
            struct_ser.serialize_field("reason", &v)?;
        }
        if let Some(v) = self.message.as_ref() {
            struct_ser.serialize_field("message", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ContentFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "reason",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Reason,
            Message,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "reason" => Ok(GeneratedField::Reason),
                            "message" => Ok(GeneratedField::Message),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ContentFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ContentFilter")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ContentFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut reason__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Reason => {
                            if reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reason"));
                            }
                            reason__ = Some(map_.next_value::<content_filter::BlockedReason>()? as i32);
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ContentFilter {
                    reason: reason__.unwrap_or_default(),
                    message: message__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ContentFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for content_filter::BlockedReason {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "BLOCKED_REASON_UNSPECIFIED",
            Self::Safety => "SAFETY",
            Self::Other => "OTHER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for content_filter::BlockedReason {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "BLOCKED_REASON_UNSPECIFIED",
            "SAFETY",
            "OTHER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = content_filter::BlockedReason;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "BLOCKED_REASON_UNSPECIFIED" => Ok(content_filter::BlockedReason::Unspecified),
                    "SAFETY" => Ok(content_filter::BlockedReason::Safety),
                    "OTHER" => Ok(content_filter::BlockedReason::Other),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ContextWindowCompressionConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.trigger_tokens.is_some() {
            len += 1;
        }
        if self.compression_mechanism.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig", len)?;
        if let Some(v) = self.trigger_tokens.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("triggerTokens", ToString::to_string(&v).as_str())?;
        }
        if let Some(v) = self.compression_mechanism.as_ref() {
            match v {
                context_window_compression_config::CompressionMechanism::SlidingWindow(v) => {
                    struct_ser.serialize_field("slidingWindow", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ContextWindowCompressionConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "trigger_tokens",
            "triggerTokens",
            "sliding_window",
            "slidingWindow",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TriggerTokens,
            SlidingWindow,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "triggerTokens" | "trigger_tokens" => Ok(GeneratedField::TriggerTokens),
                            "slidingWindow" | "sliding_window" => Ok(GeneratedField::SlidingWindow),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ContextWindowCompressionConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ContextWindowCompressionConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut trigger_tokens__ = None;
                let mut compression_mechanism__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TriggerTokens => {
                            if trigger_tokens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("triggerTokens"));
                            }
                            trigger_tokens__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::SlidingWindow => {
                            if compression_mechanism__.is_some() {
                                return Err(serde::de::Error::duplicate_field("slidingWindow"));
                            }
                            compression_mechanism__ = map_.next_value::<::std::option::Option<_>>()?.map(context_window_compression_config::CompressionMechanism::SlidingWindow)
;
                        }
                    }
                }
                Ok(ContextWindowCompressionConfig {
                    trigger_tokens: trigger_tokens__,
                    compression_mechanism: compression_mechanism__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for context_window_compression_config::SlidingWindow {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.target_tokens.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig.SlidingWindow", len)?;
        if let Some(v) = self.target_tokens.as_ref() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("targetTokens", ToString::to_string(&v).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for context_window_compression_config::SlidingWindow {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "target_tokens",
            "targetTokens",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TargetTokens,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "targetTokens" | "target_tokens" => Ok(GeneratedField::TargetTokens),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = context_window_compression_config::SlidingWindow;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig.SlidingWindow")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<context_window_compression_config::SlidingWindow, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut target_tokens__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TargetTokens => {
                            if target_tokens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("targetTokens"));
                            }
                            target_tokens__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(context_window_compression_config::SlidingWindow {
                    target_tokens: target_tokens__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ContextWindowCompressionConfig.SlidingWindow", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Corpus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.display_name.is_empty() {
            len += 1;
        }
        if self.create_time.is_some() {
            len += 1;
        }
        if self.update_time.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Corpus", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.display_name.is_empty() {
            struct_ser.serialize_field("displayName", &self.display_name)?;
        }
        if let Some(v) = self.create_time.as_ref() {
            struct_ser.serialize_field("createTime", v)?;
        }
        if let Some(v) = self.update_time.as_ref() {
            struct_ser.serialize_field("updateTime", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Corpus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "display_name",
            "displayName",
            "create_time",
            "createTime",
            "update_time",
            "updateTime",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            DisplayName,
            CreateTime,
            UpdateTime,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "displayName" | "display_name" => Ok(GeneratedField::DisplayName),
                            "createTime" | "create_time" => Ok(GeneratedField::CreateTime),
                            "updateTime" | "update_time" => Ok(GeneratedField::UpdateTime),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Corpus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Corpus")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Corpus, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut display_name__ = None;
                let mut create_time__ = None;
                let mut update_time__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DisplayName => {
                            if display_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("displayName"));
                            }
                            display_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CreateTime => {
                            if create_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createTime"));
                            }
                            create_time__ = map_.next_value()?;
                        }
                        GeneratedField::UpdateTime => {
                            if update_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateTime"));
                            }
                            update_time__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Corpus {
                    name: name__.unwrap_or_default(),
                    display_name: display_name__.unwrap_or_default(),
                    create_time: create_time__,
                    update_time: update_time__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Corpus", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CountTokensRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if !self.contents.is_empty() {
            len += 1;
        }
        if self.generate_content_request.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CountTokensRequest", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if !self.contents.is_empty() {
            struct_ser.serialize_field("contents", &self.contents)?;
        }
        if let Some(v) = self.generate_content_request.as_ref() {
            struct_ser.serialize_field("generateContentRequest", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CountTokensRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "contents",
            "generate_content_request",
            "generateContentRequest",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            Contents,
            GenerateContentRequest,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "contents" => Ok(GeneratedField::Contents),
                            "generateContentRequest" | "generate_content_request" => Ok(GeneratedField::GenerateContentRequest),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CountTokensRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CountTokensRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CountTokensRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut contents__ = None;
                let mut generate_content_request__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Contents => {
                            if contents__.is_some() {
                                return Err(serde::de::Error::duplicate_field("contents"));
                            }
                            contents__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GenerateContentRequest => {
                            if generate_content_request__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generateContentRequest"));
                            }
                            generate_content_request__ = map_.next_value()?;
                        }
                    }
                }
                Ok(CountTokensRequest {
                    model: model__.unwrap_or_default(),
                    contents: contents__.unwrap_or_default(),
                    generate_content_request: generate_content_request__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CountTokensRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CountTokensResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.total_tokens != 0 {
            len += 1;
        }
        if self.cached_content_token_count != 0 {
            len += 1;
        }
        if !self.prompt_tokens_details.is_empty() {
            len += 1;
        }
        if !self.cache_tokens_details.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CountTokensResponse", len)?;
        if self.total_tokens != 0 {
            struct_ser.serialize_field("totalTokens", &self.total_tokens)?;
        }
        if self.cached_content_token_count != 0 {
            struct_ser.serialize_field("cachedContentTokenCount", &self.cached_content_token_count)?;
        }
        if !self.prompt_tokens_details.is_empty() {
            struct_ser.serialize_field("promptTokensDetails", &self.prompt_tokens_details)?;
        }
        if !self.cache_tokens_details.is_empty() {
            struct_ser.serialize_field("cacheTokensDetails", &self.cache_tokens_details)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CountTokensResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "total_tokens",
            "totalTokens",
            "cached_content_token_count",
            "cachedContentTokenCount",
            "prompt_tokens_details",
            "promptTokensDetails",
            "cache_tokens_details",
            "cacheTokensDetails",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TotalTokens,
            CachedContentTokenCount,
            PromptTokensDetails,
            CacheTokensDetails,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "totalTokens" | "total_tokens" => Ok(GeneratedField::TotalTokens),
                            "cachedContentTokenCount" | "cached_content_token_count" => Ok(GeneratedField::CachedContentTokenCount),
                            "promptTokensDetails" | "prompt_tokens_details" => Ok(GeneratedField::PromptTokensDetails),
                            "cacheTokensDetails" | "cache_tokens_details" => Ok(GeneratedField::CacheTokensDetails),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CountTokensResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CountTokensResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CountTokensResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut total_tokens__ = None;
                let mut cached_content_token_count__ = None;
                let mut prompt_tokens_details__ = None;
                let mut cache_tokens_details__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TotalTokens => {
                            if total_tokens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalTokens"));
                            }
                            total_tokens__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CachedContentTokenCount => {
                            if cached_content_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cachedContentTokenCount"));
                            }
                            cached_content_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PromptTokensDetails => {
                            if prompt_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptTokensDetails"));
                            }
                            prompt_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CacheTokensDetails => {
                            if cache_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cacheTokensDetails"));
                            }
                            cache_tokens_details__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CountTokensResponse {
                    total_tokens: total_tokens__.unwrap_or_default(),
                    cached_content_token_count: cached_content_token_count__.unwrap_or_default(),
                    prompt_tokens_details: prompt_tokens_details__.unwrap_or_default(),
                    cache_tokens_details: cache_tokens_details__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CountTokensResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CustomMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if self.value.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.CustomMetadata", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if let Some(v) = self.value.as_ref() {
            match v {
                custom_metadata::Value::StringValue(v) => {
                    struct_ser.serialize_field("stringValue", v)?;
                }
                custom_metadata::Value::StringListValue(v) => {
                    struct_ser.serialize_field("stringListValue", v)?;
                }
                custom_metadata::Value::NumericValue(v) => {
                    struct_ser.serialize_field("numericValue", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CustomMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "string_value",
            "stringValue",
            "string_list_value",
            "stringListValue",
            "numeric_value",
            "numericValue",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            StringValue,
            StringListValue,
            NumericValue,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "stringValue" | "string_value" => Ok(GeneratedField::StringValue),
                            "stringListValue" | "string_list_value" => Ok(GeneratedField::StringListValue),
                            "numericValue" | "numeric_value" => Ok(GeneratedField::NumericValue),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CustomMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.CustomMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CustomMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StringValue => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stringValue"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(custom_metadata::Value::StringValue);
                        }
                        GeneratedField::StringListValue => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stringListValue"));
                            }
                            value__ = map_.next_value::<::std::option::Option<_>>()?.map(custom_metadata::Value::StringListValue)
;
                        }
                        GeneratedField::NumericValue => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numericValue"));
                            }
                            value__ = map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| custom_metadata::Value::NumericValue(x.0));
                        }
                    }
                }
                Ok(CustomMetadata {
                    key: key__.unwrap_or_default(),
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.CustomMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Document {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.display_name.is_empty() {
            len += 1;
        }
        if !self.custom_metadata.is_empty() {
            len += 1;
        }
        if self.update_time.is_some() {
            len += 1;
        }
        if self.create_time.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Document", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.display_name.is_empty() {
            struct_ser.serialize_field("displayName", &self.display_name)?;
        }
        if !self.custom_metadata.is_empty() {
            struct_ser.serialize_field("customMetadata", &self.custom_metadata)?;
        }
        if let Some(v) = self.update_time.as_ref() {
            struct_ser.serialize_field("updateTime", v)?;
        }
        if let Some(v) = self.create_time.as_ref() {
            struct_ser.serialize_field("createTime", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Document {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "display_name",
            "displayName",
            "custom_metadata",
            "customMetadata",
            "update_time",
            "updateTime",
            "create_time",
            "createTime",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            DisplayName,
            CustomMetadata,
            UpdateTime,
            CreateTime,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "displayName" | "display_name" => Ok(GeneratedField::DisplayName),
                            "customMetadata" | "custom_metadata" => Ok(GeneratedField::CustomMetadata),
                            "updateTime" | "update_time" => Ok(GeneratedField::UpdateTime),
                            "createTime" | "create_time" => Ok(GeneratedField::CreateTime),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Document;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Document")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Document, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut display_name__ = None;
                let mut custom_metadata__ = None;
                let mut update_time__ = None;
                let mut create_time__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DisplayName => {
                            if display_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("displayName"));
                            }
                            display_name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CustomMetadata => {
                            if custom_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("customMetadata"));
                            }
                            custom_metadata__ = Some(map_.next_value()?);
                        }
                        GeneratedField::UpdateTime => {
                            if update_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("updateTime"));
                            }
                            update_time__ = map_.next_value()?;
                        }
                        GeneratedField::CreateTime => {
                            if create_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createTime"));
                            }
                            create_time__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Document {
                    name: name__.unwrap_or_default(),
                    display_name: display_name__.unwrap_or_default(),
                    custom_metadata: custom_metadata__.unwrap_or_default(),
                    update_time: update_time__,
                    create_time: create_time__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Document", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DynamicRetrievalConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.mode != 0 {
            len += 1;
        }
        if self.dynamic_threshold.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.DynamicRetrievalConfig", len)?;
        if self.mode != 0 {
            let v = dynamic_retrieval_config::Mode::try_from(self.mode)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.mode)))?;
            struct_ser.serialize_field("mode", &v)?;
        }
        if let Some(v) = self.dynamic_threshold.as_ref() {
            struct_ser.serialize_field("dynamicThreshold", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DynamicRetrievalConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mode",
            "dynamic_threshold",
            "dynamicThreshold",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Mode,
            DynamicThreshold,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mode" => Ok(GeneratedField::Mode),
                            "dynamicThreshold" | "dynamic_threshold" => Ok(GeneratedField::DynamicThreshold),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DynamicRetrievalConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.DynamicRetrievalConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DynamicRetrievalConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mode__ = None;
                let mut dynamic_threshold__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Mode => {
                            if mode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mode"));
                            }
                            mode__ = Some(map_.next_value::<dynamic_retrieval_config::Mode>()? as i32);
                        }
                        GeneratedField::DynamicThreshold => {
                            if dynamic_threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dynamicThreshold"));
                            }
                            dynamic_threshold__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(DynamicRetrievalConfig {
                    mode: mode__.unwrap_or_default(),
                    dynamic_threshold: dynamic_threshold__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.DynamicRetrievalConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for dynamic_retrieval_config::Mode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MODE_UNSPECIFIED",
            Self::Dynamic => "MODE_DYNAMIC",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for dynamic_retrieval_config::Mode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MODE_UNSPECIFIED",
            "MODE_DYNAMIC",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = dynamic_retrieval_config::Mode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MODE_UNSPECIFIED" => Ok(dynamic_retrieval_config::Mode::Unspecified),
                    "MODE_DYNAMIC" => Ok(dynamic_retrieval_config::Mode::Dynamic),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for EmbedContentRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        if self.task_type.is_some() {
            len += 1;
        }
        if self.title.is_some() {
            len += 1;
        }
        if self.output_dimensionality.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.EmbedContentRequest", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if let Some(v) = self.content.as_ref() {
            struct_ser.serialize_field("content", v)?;
        }
        if let Some(v) = self.task_type.as_ref() {
            let v = TaskType::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("taskType", &v)?;
        }
        if let Some(v) = self.title.as_ref() {
            struct_ser.serialize_field("title", v)?;
        }
        if let Some(v) = self.output_dimensionality.as_ref() {
            struct_ser.serialize_field("outputDimensionality", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EmbedContentRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "content",
            "task_type",
            "taskType",
            "title",
            "output_dimensionality",
            "outputDimensionality",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            Content,
            TaskType,
            Title,
            OutputDimensionality,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "content" => Ok(GeneratedField::Content),
                            "taskType" | "task_type" => Ok(GeneratedField::TaskType),
                            "title" => Ok(GeneratedField::Title),
                            "outputDimensionality" | "output_dimensionality" => Ok(GeneratedField::OutputDimensionality),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EmbedContentRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.EmbedContentRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EmbedContentRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut content__ = None;
                let mut task_type__ = None;
                let mut title__ = None;
                let mut output_dimensionality__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = map_.next_value()?;
                        }
                        GeneratedField::TaskType => {
                            if task_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("taskType"));
                            }
                            task_type__ = map_.next_value::<::std::option::Option<TaskType>>()?.map(|x| x as i32);
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = map_.next_value()?;
                        }
                        GeneratedField::OutputDimensionality => {
                            if output_dimensionality__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputDimensionality"));
                            }
                            output_dimensionality__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(EmbedContentRequest {
                    model: model__.unwrap_or_default(),
                    content: content__,
                    task_type: task_type__,
                    title: title__,
                    output_dimensionality: output_dimensionality__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.EmbedContentRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for EmbedContentResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.embedding.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.EmbedContentResponse", len)?;
        if let Some(v) = self.embedding.as_ref() {
            struct_ser.serialize_field("embedding", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for EmbedContentResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "embedding",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Embedding,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "embedding" => Ok(GeneratedField::Embedding),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = EmbedContentResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.EmbedContentResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<EmbedContentResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut embedding__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Embedding => {
                            if embedding__.is_some() {
                                return Err(serde::de::Error::duplicate_field("embedding"));
                            }
                            embedding__ = map_.next_value()?;
                        }
                    }
                }
                Ok(EmbedContentResponse {
                    embedding: embedding__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.EmbedContentResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExecutableCode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.language != 0 {
            len += 1;
        }
        if !self.code.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ExecutableCode", len)?;
        if self.language != 0 {
            let v = executable_code::Language::try_from(self.language)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.language)))?;
            struct_ser.serialize_field("language", &v)?;
        }
        if !self.code.is_empty() {
            struct_ser.serialize_field("code", &self.code)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExecutableCode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "language",
            "code",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Language,
            Code,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "language" => Ok(GeneratedField::Language),
                            "code" => Ok(GeneratedField::Code),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExecutableCode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ExecutableCode")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExecutableCode, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut language__ = None;
                let mut code__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Language => {
                            if language__.is_some() {
                                return Err(serde::de::Error::duplicate_field("language"));
                            }
                            language__ = Some(map_.next_value::<executable_code::Language>()? as i32);
                        }
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ExecutableCode {
                    language: language__.unwrap_or_default(),
                    code: code__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ExecutableCode", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for executable_code::Language {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "LANGUAGE_UNSPECIFIED",
            Self::Python => "PYTHON",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for executable_code::Language {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "LANGUAGE_UNSPECIFIED",
            "PYTHON",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = executable_code::Language;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "LANGUAGE_UNSPECIFIED" => Ok(executable_code::Language::Unspecified),
                    "PYTHON" => Ok(executable_code::Language::Python),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for FileData {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.mime_type.is_empty() {
            len += 1;
        }
        if !self.file_uri.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FileData", len)?;
        if !self.mime_type.is_empty() {
            struct_ser.serialize_field("mimeType", &self.mime_type)?;
        }
        if !self.file_uri.is_empty() {
            struct_ser.serialize_field("fileUri", &self.file_uri)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FileData {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mime_type",
            "mimeType",
            "file_uri",
            "fileUri",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MimeType,
            FileUri,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mimeType" | "mime_type" => Ok(GeneratedField::MimeType),
                            "fileUri" | "file_uri" => Ok(GeneratedField::FileUri),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FileData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FileData")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FileData, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mime_type__ = None;
                let mut file_uri__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MimeType => {
                            if mime_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mimeType"));
                            }
                            mime_type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::FileUri => {
                            if file_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fileUri"));
                            }
                            file_uri__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(FileData {
                    mime_type: mime_type__.unwrap_or_default(),
                    file_uri: file_uri__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FileData", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FileSearch {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.retrieval_resources.is_empty() {
            len += 1;
        }
        if self.retrieval_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FileSearch", len)?;
        if !self.retrieval_resources.is_empty() {
            struct_ser.serialize_field("retrievalResources", &self.retrieval_resources)?;
        }
        if let Some(v) = self.retrieval_config.as_ref() {
            struct_ser.serialize_field("retrievalConfig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FileSearch {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "retrieval_resources",
            "retrievalResources",
            "retrieval_config",
            "retrievalConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RetrievalResources,
            RetrievalConfig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "retrievalResources" | "retrieval_resources" => Ok(GeneratedField::RetrievalResources),
                            "retrievalConfig" | "retrieval_config" => Ok(GeneratedField::RetrievalConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FileSearch;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FileSearch")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FileSearch, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut retrieval_resources__ = None;
                let mut retrieval_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RetrievalResources => {
                            if retrieval_resources__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievalResources"));
                            }
                            retrieval_resources__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RetrievalConfig => {
                            if retrieval_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievalConfig"));
                            }
                            retrieval_config__ = map_.next_value()?;
                        }
                    }
                }
                Ok(FileSearch {
                    retrieval_resources: retrieval_resources__.unwrap_or_default(),
                    retrieval_config: retrieval_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FileSearch", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for file_search::RetrievalConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.top_k.is_some() {
            len += 1;
        }
        if !self.metadata_filter.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FileSearch.RetrievalConfig", len)?;
        if let Some(v) = self.top_k.as_ref() {
            struct_ser.serialize_field("topK", v)?;
        }
        if !self.metadata_filter.is_empty() {
            struct_ser.serialize_field("metadataFilter", &self.metadata_filter)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for file_search::RetrievalConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "top_k",
            "topK",
            "metadata_filter",
            "metadataFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TopK,
            MetadataFilter,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "topK" | "top_k" => Ok(GeneratedField::TopK),
                            "metadataFilter" | "metadata_filter" => Ok(GeneratedField::MetadataFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = file_search::RetrievalConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FileSearch.RetrievalConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<file_search::RetrievalConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut top_k__ = None;
                let mut metadata_filter__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TopK => {
                            if top_k__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topK"));
                            }
                            top_k__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::MetadataFilter => {
                            if metadata_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataFilter"));
                            }
                            metadata_filter__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(file_search::RetrievalConfig {
                    top_k: top_k__,
                    metadata_filter: metadata_filter__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FileSearch.RetrievalConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for file_search::RetrievalResource {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rag_store_name.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FileSearch.RetrievalResource", len)?;
        if !self.rag_store_name.is_empty() {
            struct_ser.serialize_field("ragStoreName", &self.rag_store_name)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for file_search::RetrievalResource {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rag_store_name",
            "ragStoreName",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RagStoreName,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "ragStoreName" | "rag_store_name" => Ok(GeneratedField::RagStoreName),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = file_search::RetrievalResource;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FileSearch.RetrievalResource")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<file_search::RetrievalResource, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rag_store_name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RagStoreName => {
                            if rag_store_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ragStoreName"));
                            }
                            rag_store_name__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(file_search::RetrievalResource {
                    rag_store_name: rag_store_name__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FileSearch.RetrievalResource", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionCall {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if self.args.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionCall", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if let Some(v) = self.args.as_ref() {
            struct_ser.serialize_field("args", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionCall {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "name",
            "args",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Name,
            Args,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            "args" => Ok(GeneratedField::Args),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionCall;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionCall")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionCall, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut name__ = None;
                let mut args__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Args => {
                            if args__.is_some() {
                                return Err(serde::de::Error::duplicate_field("args"));
                            }
                            args__ = map_.next_value()?;
                        }
                    }
                }
                Ok(FunctionCall {
                    id: id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    args: args__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionCall", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionCallingConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.mode != 0 {
            len += 1;
        }
        if !self.allowed_function_names.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionCallingConfig", len)?;
        if self.mode != 0 {
            let v = function_calling_config::Mode::try_from(self.mode)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.mode)))?;
            struct_ser.serialize_field("mode", &v)?;
        }
        if !self.allowed_function_names.is_empty() {
            struct_ser.serialize_field("allowedFunctionNames", &self.allowed_function_names)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionCallingConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mode",
            "allowed_function_names",
            "allowedFunctionNames",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Mode,
            AllowedFunctionNames,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mode" => Ok(GeneratedField::Mode),
                            "allowedFunctionNames" | "allowed_function_names" => Ok(GeneratedField::AllowedFunctionNames),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionCallingConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionCallingConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionCallingConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mode__ = None;
                let mut allowed_function_names__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Mode => {
                            if mode__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mode"));
                            }
                            mode__ = Some(map_.next_value::<function_calling_config::Mode>()? as i32);
                        }
                        GeneratedField::AllowedFunctionNames => {
                            if allowed_function_names__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allowedFunctionNames"));
                            }
                            allowed_function_names__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(FunctionCallingConfig {
                    mode: mode__.unwrap_or_default(),
                    allowed_function_names: allowed_function_names__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionCallingConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for function_calling_config::Mode {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MODE_UNSPECIFIED",
            Self::Auto => "AUTO",
            Self::Any => "ANY",
            Self::None => "NONE",
            Self::Validated => "VALIDATED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for function_calling_config::Mode {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MODE_UNSPECIFIED",
            "AUTO",
            "ANY",
            "NONE",
            "VALIDATED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = function_calling_config::Mode;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MODE_UNSPECIFIED" => Ok(function_calling_config::Mode::Unspecified),
                    "AUTO" => Ok(function_calling_config::Mode::Auto),
                    "ANY" => Ok(function_calling_config::Mode::Any),
                    "NONE" => Ok(function_calling_config::Mode::None),
                    "VALIDATED" => Ok(function_calling_config::Mode::Validated),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionDeclaration {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.name.is_empty() {
            len += 1;
        }
        if !self.description.is_empty() {
            len += 1;
        }
        if self.parameters.is_some() {
            len += 1;
        }
        if self.parameters_json_schema.is_some() {
            len += 1;
        }
        if self.response.is_some() {
            len += 1;
        }
        if self.response_json_schema.is_some() {
            len += 1;
        }
        if self.behavior != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionDeclaration", len)?;
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if !self.description.is_empty() {
            struct_ser.serialize_field("description", &self.description)?;
        }
        if let Some(v) = self.parameters.as_ref() {
            struct_ser.serialize_field("parameters", v)?;
        }
        if let Some(v) = self.parameters_json_schema.as_ref() {
            struct_ser.serialize_field("parametersJsonSchema", v)?;
        }
        if let Some(v) = self.response.as_ref() {
            struct_ser.serialize_field("response", v)?;
        }
        if let Some(v) = self.response_json_schema.as_ref() {
            struct_ser.serialize_field("responseJsonSchema", v)?;
        }
        if self.behavior != 0 {
            let v = function_declaration::Behavior::try_from(self.behavior)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.behavior)))?;
            struct_ser.serialize_field("behavior", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionDeclaration {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "description",
            "parameters",
            "parameters_json_schema",
            "parametersJsonSchema",
            "response",
            "response_json_schema",
            "responseJsonSchema",
            "behavior",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Description,
            Parameters,
            ParametersJsonSchema,
            Response,
            ResponseJsonSchema,
            Behavior,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "name" => Ok(GeneratedField::Name),
                            "description" => Ok(GeneratedField::Description),
                            "parameters" => Ok(GeneratedField::Parameters),
                            "parametersJsonSchema" | "parameters_json_schema" => Ok(GeneratedField::ParametersJsonSchema),
                            "response" => Ok(GeneratedField::Response),
                            "responseJsonSchema" | "response_json_schema" => Ok(GeneratedField::ResponseJsonSchema),
                            "behavior" => Ok(GeneratedField::Behavior),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionDeclaration;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionDeclaration")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionDeclaration, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut description__ = None;
                let mut parameters__ = None;
                let mut parameters_json_schema__ = None;
                let mut response__ = None;
                let mut response_json_schema__ = None;
                let mut behavior__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Description => {
                            if description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("description"));
                            }
                            description__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Parameters => {
                            if parameters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parameters"));
                            }
                            parameters__ = map_.next_value()?;
                        }
                        GeneratedField::ParametersJsonSchema => {
                            if parameters_json_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parametersJsonSchema"));
                            }
                            parameters_json_schema__ = map_.next_value()?;
                        }
                        GeneratedField::Response => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("response"));
                            }
                            response__ = map_.next_value()?;
                        }
                        GeneratedField::ResponseJsonSchema => {
                            if response_json_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseJsonSchema"));
                            }
                            response_json_schema__ = map_.next_value()?;
                        }
                        GeneratedField::Behavior => {
                            if behavior__.is_some() {
                                return Err(serde::de::Error::duplicate_field("behavior"));
                            }
                            behavior__ = Some(map_.next_value::<function_declaration::Behavior>()? as i32);
                        }
                    }
                }
                Ok(FunctionDeclaration {
                    name: name__.unwrap_or_default(),
                    description: description__.unwrap_or_default(),
                    parameters: parameters__,
                    parameters_json_schema: parameters_json_schema__,
                    response: response__,
                    response_json_schema: response_json_schema__,
                    behavior: behavior__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionDeclaration", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for function_declaration::Behavior {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Blocking => "BLOCKING",
            Self::NonBlocking => "NON_BLOCKING",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for function_declaration::Behavior {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "UNSPECIFIED",
            "BLOCKING",
            "NON_BLOCKING",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = function_declaration::Behavior;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "UNSPECIFIED" => Ok(function_declaration::Behavior::Unspecified),
                    "BLOCKING" => Ok(function_declaration::Behavior::Blocking),
                    "NON_BLOCKING" => Ok(function_declaration::Behavior::NonBlocking),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if !self.name.is_empty() {
            len += 1;
        }
        if self.response.is_some() {
            len += 1;
        }
        if !self.parts.is_empty() {
            len += 1;
        }
        if self.will_continue {
            len += 1;
        }
        if self.scheduling.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionResponse", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if !self.name.is_empty() {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if let Some(v) = self.response.as_ref() {
            struct_ser.serialize_field("response", v)?;
        }
        if !self.parts.is_empty() {
            struct_ser.serialize_field("parts", &self.parts)?;
        }
        if self.will_continue {
            struct_ser.serialize_field("willContinue", &self.will_continue)?;
        }
        if let Some(v) = self.scheduling.as_ref() {
            let v = function_response::Scheduling::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("scheduling", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "name",
            "response",
            "parts",
            "will_continue",
            "willContinue",
            "scheduling",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Name,
            Response,
            Parts,
            WillContinue,
            Scheduling,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "name" => Ok(GeneratedField::Name),
                            "response" => Ok(GeneratedField::Response),
                            "parts" => Ok(GeneratedField::Parts),
                            "willContinue" | "will_continue" => Ok(GeneratedField::WillContinue),
                            "scheduling" => Ok(GeneratedField::Scheduling),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut name__ = None;
                let mut response__ = None;
                let mut parts__ = None;
                let mut will_continue__ = None;
                let mut scheduling__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Response => {
                            if response__.is_some() {
                                return Err(serde::de::Error::duplicate_field("response"));
                            }
                            response__ = map_.next_value()?;
                        }
                        GeneratedField::Parts => {
                            if parts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parts"));
                            }
                            parts__ = Some(map_.next_value()?);
                        }
                        GeneratedField::WillContinue => {
                            if will_continue__.is_some() {
                                return Err(serde::de::Error::duplicate_field("willContinue"));
                            }
                            will_continue__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Scheduling => {
                            if scheduling__.is_some() {
                                return Err(serde::de::Error::duplicate_field("scheduling"));
                            }
                            scheduling__ = map_.next_value::<::std::option::Option<function_response::Scheduling>>()?.map(|x| x as i32);
                        }
                    }
                }
                Ok(FunctionResponse {
                    id: id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    response: response__,
                    parts: parts__.unwrap_or_default(),
                    will_continue: will_continue__.unwrap_or_default(),
                    scheduling: scheduling__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for function_response::Scheduling {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "SCHEDULING_UNSPECIFIED",
            Self::Silent => "SILENT",
            Self::WhenIdle => "WHEN_IDLE",
            Self::Interrupt => "INTERRUPT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for function_response::Scheduling {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "SCHEDULING_UNSPECIFIED",
            "SILENT",
            "WHEN_IDLE",
            "INTERRUPT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = function_response::Scheduling;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "SCHEDULING_UNSPECIFIED" => Ok(function_response::Scheduling::Unspecified),
                    "SILENT" => Ok(function_response::Scheduling::Silent),
                    "WHEN_IDLE" => Ok(function_response::Scheduling::WhenIdle),
                    "INTERRUPT" => Ok(function_response::Scheduling::Interrupt),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionResponseBlob {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.mime_type.is_empty() {
            len += 1;
        }
        if !self.data.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionResponseBlob", len)?;
        if !self.mime_type.is_empty() {
            struct_ser.serialize_field("mimeType", &self.mime_type)?;
        }
        if !self.data.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("data", pbjson::private::base64::encode(&self.data).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionResponseBlob {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "mime_type",
            "mimeType",
            "data",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            MimeType,
            Data,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "mimeType" | "mime_type" => Ok(GeneratedField::MimeType),
                            "data" => Ok(GeneratedField::Data),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionResponseBlob;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionResponseBlob")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionResponseBlob, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut mime_type__ = None;
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::MimeType => {
                            if mime_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mimeType"));
                            }
                            mime_type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Data => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(FunctionResponseBlob {
                    mime_type: mime_type__.unwrap_or_default(),
                    data: data__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionResponseBlob", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for FunctionResponsePart {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.data.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.FunctionResponsePart", len)?;
        if let Some(v) = self.data.as_ref() {
            match v {
                function_response_part::Data::InlineData(v) => {
                    struct_ser.serialize_field("inlineData", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for FunctionResponsePart {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "inline_data",
            "inlineData",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            InlineData,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "inlineData" | "inline_data" => Ok(GeneratedField::InlineData),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = FunctionResponsePart;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.FunctionResponsePart")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<FunctionResponsePart, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut data__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::InlineData => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inlineData"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(function_response_part::Data::InlineData)
;
                        }
                    }
                }
                Ok(FunctionResponsePart {
                    data: data__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.FunctionResponsePart", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenerateAnswerRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if !self.contents.is_empty() {
            len += 1;
        }
        if self.answer_style != 0 {
            len += 1;
        }
        if !self.safety_settings.is_empty() {
            len += 1;
        }
        if self.temperature.is_some() {
            len += 1;
        }
        if self.grounding_source.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerRequest", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if !self.contents.is_empty() {
            struct_ser.serialize_field("contents", &self.contents)?;
        }
        if self.answer_style != 0 {
            let v = generate_answer_request::AnswerStyle::try_from(self.answer_style)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.answer_style)))?;
            struct_ser.serialize_field("answerStyle", &v)?;
        }
        if !self.safety_settings.is_empty() {
            struct_ser.serialize_field("safetySettings", &self.safety_settings)?;
        }
        if let Some(v) = self.temperature.as_ref() {
            struct_ser.serialize_field("temperature", v)?;
        }
        if let Some(v) = self.grounding_source.as_ref() {
            match v {
                generate_answer_request::GroundingSource::InlinePassages(v) => {
                    struct_ser.serialize_field("inlinePassages", v)?;
                }
                generate_answer_request::GroundingSource::SemanticRetriever(v) => {
                    struct_ser.serialize_field("semanticRetriever", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenerateAnswerRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "contents",
            "answer_style",
            "answerStyle",
            "safety_settings",
            "safetySettings",
            "temperature",
            "inline_passages",
            "inlinePassages",
            "semantic_retriever",
            "semanticRetriever",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            Contents,
            AnswerStyle,
            SafetySettings,
            Temperature,
            InlinePassages,
            SemanticRetriever,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "contents" => Ok(GeneratedField::Contents),
                            "answerStyle" | "answer_style" => Ok(GeneratedField::AnswerStyle),
                            "safetySettings" | "safety_settings" => Ok(GeneratedField::SafetySettings),
                            "temperature" => Ok(GeneratedField::Temperature),
                            "inlinePassages" | "inline_passages" => Ok(GeneratedField::InlinePassages),
                            "semanticRetriever" | "semantic_retriever" => Ok(GeneratedField::SemanticRetriever),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenerateAnswerRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateAnswerRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenerateAnswerRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut contents__ = None;
                let mut answer_style__ = None;
                let mut safety_settings__ = None;
                let mut temperature__ = None;
                let mut grounding_source__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Contents => {
                            if contents__.is_some() {
                                return Err(serde::de::Error::duplicate_field("contents"));
                            }
                            contents__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AnswerStyle => {
                            if answer_style__.is_some() {
                                return Err(serde::de::Error::duplicate_field("answerStyle"));
                            }
                            answer_style__ = Some(map_.next_value::<generate_answer_request::AnswerStyle>()? as i32);
                        }
                        GeneratedField::SafetySettings => {
                            if safety_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("safetySettings"));
                            }
                            safety_settings__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Temperature => {
                            if temperature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("temperature"));
                            }
                            temperature__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::InlinePassages => {
                            if grounding_source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inlinePassages"));
                            }
                            grounding_source__ = map_.next_value::<::std::option::Option<_>>()?.map(generate_answer_request::GroundingSource::InlinePassages)
;
                        }
                        GeneratedField::SemanticRetriever => {
                            if grounding_source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("semanticRetriever"));
                            }
                            grounding_source__ = map_.next_value::<::std::option::Option<_>>()?.map(generate_answer_request::GroundingSource::SemanticRetriever)
;
                        }
                    }
                }
                Ok(GenerateAnswerRequest {
                    model: model__.unwrap_or_default(),
                    contents: contents__.unwrap_or_default(),
                    answer_style: answer_style__.unwrap_or_default(),
                    safety_settings: safety_settings__.unwrap_or_default(),
                    temperature: temperature__,
                    grounding_source: grounding_source__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generate_answer_request::AnswerStyle {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ANSWER_STYLE_UNSPECIFIED",
            Self::Abstractive => "ABSTRACTIVE",
            Self::Extractive => "EXTRACTIVE",
            Self::Verbose => "VERBOSE",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for generate_answer_request::AnswerStyle {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ANSWER_STYLE_UNSPECIFIED",
            "ABSTRACTIVE",
            "EXTRACTIVE",
            "VERBOSE",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_answer_request::AnswerStyle;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ANSWER_STYLE_UNSPECIFIED" => Ok(generate_answer_request::AnswerStyle::Unspecified),
                    "ABSTRACTIVE" => Ok(generate_answer_request::AnswerStyle::Abstractive),
                    "EXTRACTIVE" => Ok(generate_answer_request::AnswerStyle::Extractive),
                    "VERBOSE" => Ok(generate_answer_request::AnswerStyle::Verbose),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for GenerateAnswerResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.answer.is_some() {
            len += 1;
        }
        if self.answerable_probability.is_some() {
            len += 1;
        }
        if self.input_feedback.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerResponse", len)?;
        if let Some(v) = self.answer.as_ref() {
            struct_ser.serialize_field("answer", v)?;
        }
        if let Some(v) = self.answerable_probability.as_ref() {
            struct_ser.serialize_field("answerableProbability", v)?;
        }
        if let Some(v) = self.input_feedback.as_ref() {
            struct_ser.serialize_field("inputFeedback", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenerateAnswerResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "answer",
            "answerable_probability",
            "answerableProbability",
            "input_feedback",
            "inputFeedback",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Answer,
            AnswerableProbability,
            InputFeedback,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "answer" => Ok(GeneratedField::Answer),
                            "answerableProbability" | "answerable_probability" => Ok(GeneratedField::AnswerableProbability),
                            "inputFeedback" | "input_feedback" => Ok(GeneratedField::InputFeedback),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenerateAnswerResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateAnswerResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenerateAnswerResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut answer__ = None;
                let mut answerable_probability__ = None;
                let mut input_feedback__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Answer => {
                            if answer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("answer"));
                            }
                            answer__ = map_.next_value()?;
                        }
                        GeneratedField::AnswerableProbability => {
                            if answerable_probability__.is_some() {
                                return Err(serde::de::Error::duplicate_field("answerableProbability"));
                            }
                            answerable_probability__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::InputFeedback => {
                            if input_feedback__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inputFeedback"));
                            }
                            input_feedback__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GenerateAnswerResponse {
                    answer: answer__,
                    answerable_probability: answerable_probability__,
                    input_feedback: input_feedback__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generate_answer_response::InputFeedback {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_reason.is_some() {
            len += 1;
        }
        if !self.safety_ratings.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerResponse.InputFeedback", len)?;
        if let Some(v) = self.block_reason.as_ref() {
            let v = generate_answer_response::input_feedback::BlockReason::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("blockReason", &v)?;
        }
        if !self.safety_ratings.is_empty() {
            struct_ser.serialize_field("safetyRatings", &self.safety_ratings)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for generate_answer_response::InputFeedback {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_reason",
            "blockReason",
            "safety_ratings",
            "safetyRatings",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockReason,
            SafetyRatings,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blockReason" | "block_reason" => Ok(GeneratedField::BlockReason),
                            "safetyRatings" | "safety_ratings" => Ok(GeneratedField::SafetyRatings),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_answer_response::InputFeedback;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateAnswerResponse.InputFeedback")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<generate_answer_response::InputFeedback, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_reason__ = None;
                let mut safety_ratings__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockReason => {
                            if block_reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockReason"));
                            }
                            block_reason__ = map_.next_value::<::std::option::Option<generate_answer_response::input_feedback::BlockReason>>()?.map(|x| x as i32);
                        }
                        GeneratedField::SafetyRatings => {
                            if safety_ratings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("safetyRatings"));
                            }
                            safety_ratings__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(generate_answer_response::InputFeedback {
                    block_reason: block_reason__,
                    safety_ratings: safety_ratings__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateAnswerResponse.InputFeedback", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generate_answer_response::input_feedback::BlockReason {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "BLOCK_REASON_UNSPECIFIED",
            Self::Safety => "SAFETY",
            Self::Other => "OTHER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for generate_answer_response::input_feedback::BlockReason {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "BLOCK_REASON_UNSPECIFIED",
            "SAFETY",
            "OTHER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_answer_response::input_feedback::BlockReason;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "BLOCK_REASON_UNSPECIFIED" => Ok(generate_answer_response::input_feedback::BlockReason::Unspecified),
                    "SAFETY" => Ok(generate_answer_response::input_feedback::BlockReason::Safety),
                    "OTHER" => Ok(generate_answer_response::input_feedback::BlockReason::Other),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for GenerateContentRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.model.is_empty() {
            len += 1;
        }
        if self.system_instruction.is_some() {
            len += 1;
        }
        if !self.contents.is_empty() {
            len += 1;
        }
        if !self.tools.is_empty() {
            len += 1;
        }
        if self.tool_config.is_some() {
            len += 1;
        }
        if !self.safety_settings.is_empty() {
            len += 1;
        }
        if self.generation_config.is_some() {
            len += 1;
        }
        if self.cached_content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateContentRequest", len)?;
        if !self.model.is_empty() {
            struct_ser.serialize_field("model", &self.model)?;
        }
        if let Some(v) = self.system_instruction.as_ref() {
            struct_ser.serialize_field("systemInstruction", v)?;
        }
        if !self.contents.is_empty() {
            struct_ser.serialize_field("contents", &self.contents)?;
        }
        if !self.tools.is_empty() {
            struct_ser.serialize_field("tools", &self.tools)?;
        }
        if let Some(v) = self.tool_config.as_ref() {
            struct_ser.serialize_field("toolConfig", v)?;
        }
        if !self.safety_settings.is_empty() {
            struct_ser.serialize_field("safetySettings", &self.safety_settings)?;
        }
        if let Some(v) = self.generation_config.as_ref() {
            struct_ser.serialize_field("generationConfig", v)?;
        }
        if let Some(v) = self.cached_content.as_ref() {
            struct_ser.serialize_field("cachedContent", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenerateContentRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model",
            "system_instruction",
            "systemInstruction",
            "contents",
            "tools",
            "tool_config",
            "toolConfig",
            "safety_settings",
            "safetySettings",
            "generation_config",
            "generationConfig",
            "cached_content",
            "cachedContent",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Model,
            SystemInstruction,
            Contents,
            Tools,
            ToolConfig,
            SafetySettings,
            GenerationConfig,
            CachedContent,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "model" => Ok(GeneratedField::Model),
                            "systemInstruction" | "system_instruction" => Ok(GeneratedField::SystemInstruction),
                            "contents" => Ok(GeneratedField::Contents),
                            "tools" => Ok(GeneratedField::Tools),
                            "toolConfig" | "tool_config" => Ok(GeneratedField::ToolConfig),
                            "safetySettings" | "safety_settings" => Ok(GeneratedField::SafetySettings),
                            "generationConfig" | "generation_config" => Ok(GeneratedField::GenerationConfig),
                            "cachedContent" | "cached_content" => Ok(GeneratedField::CachedContent),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenerateContentRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateContentRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenerateContentRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model__ = None;
                let mut system_instruction__ = None;
                let mut contents__ = None;
                let mut tools__ = None;
                let mut tool_config__ = None;
                let mut safety_settings__ = None;
                let mut generation_config__ = None;
                let mut cached_content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Model => {
                            if model__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            model__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SystemInstruction => {
                            if system_instruction__.is_some() {
                                return Err(serde::de::Error::duplicate_field("systemInstruction"));
                            }
                            system_instruction__ = map_.next_value()?;
                        }
                        GeneratedField::Contents => {
                            if contents__.is_some() {
                                return Err(serde::de::Error::duplicate_field("contents"));
                            }
                            contents__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tools => {
                            if tools__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tools"));
                            }
                            tools__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ToolConfig => {
                            if tool_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolConfig"));
                            }
                            tool_config__ = map_.next_value()?;
                        }
                        GeneratedField::SafetySettings => {
                            if safety_settings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("safetySettings"));
                            }
                            safety_settings__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GenerationConfig => {
                            if generation_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generationConfig"));
                            }
                            generation_config__ = map_.next_value()?;
                        }
                        GeneratedField::CachedContent => {
                            if cached_content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cachedContent"));
                            }
                            cached_content__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GenerateContentRequest {
                    model: model__.unwrap_or_default(),
                    system_instruction: system_instruction__,
                    contents: contents__.unwrap_or_default(),
                    tools: tools__.unwrap_or_default(),
                    tool_config: tool_config__,
                    safety_settings: safety_settings__.unwrap_or_default(),
                    generation_config: generation_config__,
                    cached_content: cached_content__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateContentRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenerateContentResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.candidates.is_empty() {
            len += 1;
        }
        if self.prompt_feedback.is_some() {
            len += 1;
        }
        if self.usage_metadata.is_some() {
            len += 1;
        }
        if !self.model_version.is_empty() {
            len += 1;
        }
        if !self.response_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse", len)?;
        if !self.candidates.is_empty() {
            struct_ser.serialize_field("candidates", &self.candidates)?;
        }
        if let Some(v) = self.prompt_feedback.as_ref() {
            struct_ser.serialize_field("promptFeedback", v)?;
        }
        if let Some(v) = self.usage_metadata.as_ref() {
            struct_ser.serialize_field("usageMetadata", v)?;
        }
        if !self.model_version.is_empty() {
            struct_ser.serialize_field("modelVersion", &self.model_version)?;
        }
        if !self.response_id.is_empty() {
            struct_ser.serialize_field("responseId", &self.response_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenerateContentResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "candidates",
            "prompt_feedback",
            "promptFeedback",
            "usage_metadata",
            "usageMetadata",
            "model_version",
            "modelVersion",
            "response_id",
            "responseId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Candidates,
            PromptFeedback,
            UsageMetadata,
            ModelVersion,
            ResponseId,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "candidates" => Ok(GeneratedField::Candidates),
                            "promptFeedback" | "prompt_feedback" => Ok(GeneratedField::PromptFeedback),
                            "usageMetadata" | "usage_metadata" => Ok(GeneratedField::UsageMetadata),
                            "modelVersion" | "model_version" => Ok(GeneratedField::ModelVersion),
                            "responseId" | "response_id" => Ok(GeneratedField::ResponseId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenerateContentResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateContentResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenerateContentResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut candidates__ = None;
                let mut prompt_feedback__ = None;
                let mut usage_metadata__ = None;
                let mut model_version__ = None;
                let mut response_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Candidates => {
                            if candidates__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidates"));
                            }
                            candidates__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PromptFeedback => {
                            if prompt_feedback__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptFeedback"));
                            }
                            prompt_feedback__ = map_.next_value()?;
                        }
                        GeneratedField::UsageMetadata => {
                            if usage_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("usageMetadata"));
                            }
                            usage_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::ModelVersion => {
                            if model_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelVersion"));
                            }
                            model_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ResponseId => {
                            if response_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseId"));
                            }
                            response_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GenerateContentResponse {
                    candidates: candidates__.unwrap_or_default(),
                    prompt_feedback: prompt_feedback__,
                    usage_metadata: usage_metadata__,
                    model_version: model_version__.unwrap_or_default(),
                    response_id: response_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generate_content_response::PromptFeedback {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.block_reason != 0 {
            len += 1;
        }
        if !self.safety_ratings.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse.PromptFeedback", len)?;
        if self.block_reason != 0 {
            let v = generate_content_response::prompt_feedback::BlockReason::try_from(self.block_reason)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.block_reason)))?;
            struct_ser.serialize_field("blockReason", &v)?;
        }
        if !self.safety_ratings.is_empty() {
            struct_ser.serialize_field("safetyRatings", &self.safety_ratings)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for generate_content_response::PromptFeedback {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "block_reason",
            "blockReason",
            "safety_ratings",
            "safetyRatings",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            BlockReason,
            SafetyRatings,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "blockReason" | "block_reason" => Ok(GeneratedField::BlockReason),
                            "safetyRatings" | "safety_ratings" => Ok(GeneratedField::SafetyRatings),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_content_response::PromptFeedback;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateContentResponse.PromptFeedback")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<generate_content_response::PromptFeedback, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut block_reason__ = None;
                let mut safety_ratings__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::BlockReason => {
                            if block_reason__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blockReason"));
                            }
                            block_reason__ = Some(map_.next_value::<generate_content_response::prompt_feedback::BlockReason>()? as i32);
                        }
                        GeneratedField::SafetyRatings => {
                            if safety_ratings__.is_some() {
                                return Err(serde::de::Error::duplicate_field("safetyRatings"));
                            }
                            safety_ratings__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(generate_content_response::PromptFeedback {
                    block_reason: block_reason__.unwrap_or_default(),
                    safety_ratings: safety_ratings__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse.PromptFeedback", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generate_content_response::prompt_feedback::BlockReason {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "BLOCK_REASON_UNSPECIFIED",
            Self::Safety => "SAFETY",
            Self::Other => "OTHER",
            Self::Blocklist => "BLOCKLIST",
            Self::ProhibitedContent => "PROHIBITED_CONTENT",
            Self::ImageSafety => "IMAGE_SAFETY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for generate_content_response::prompt_feedback::BlockReason {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "BLOCK_REASON_UNSPECIFIED",
            "SAFETY",
            "OTHER",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "IMAGE_SAFETY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_content_response::prompt_feedback::BlockReason;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "BLOCK_REASON_UNSPECIFIED" => Ok(generate_content_response::prompt_feedback::BlockReason::Unspecified),
                    "SAFETY" => Ok(generate_content_response::prompt_feedback::BlockReason::Safety),
                    "OTHER" => Ok(generate_content_response::prompt_feedback::BlockReason::Other),
                    "BLOCKLIST" => Ok(generate_content_response::prompt_feedback::BlockReason::Blocklist),
                    "PROHIBITED_CONTENT" => Ok(generate_content_response::prompt_feedback::BlockReason::ProhibitedContent),
                    "IMAGE_SAFETY" => Ok(generate_content_response::prompt_feedback::BlockReason::ImageSafety),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for generate_content_response::UsageMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.prompt_token_count != 0 {
            len += 1;
        }
        if self.cached_content_token_count != 0 {
            len += 1;
        }
        if self.candidates_token_count != 0 {
            len += 1;
        }
        if self.tool_use_prompt_token_count != 0 {
            len += 1;
        }
        if self.thoughts_token_count != 0 {
            len += 1;
        }
        if self.total_token_count != 0 {
            len += 1;
        }
        if !self.prompt_tokens_details.is_empty() {
            len += 1;
        }
        if !self.cache_tokens_details.is_empty() {
            len += 1;
        }
        if !self.candidates_tokens_details.is_empty() {
            len += 1;
        }
        if !self.tool_use_prompt_tokens_details.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse.UsageMetadata", len)?;
        if self.prompt_token_count != 0 {
            struct_ser.serialize_field("promptTokenCount", &self.prompt_token_count)?;
        }
        if self.cached_content_token_count != 0 {
            struct_ser.serialize_field("cachedContentTokenCount", &self.cached_content_token_count)?;
        }
        if self.candidates_token_count != 0 {
            struct_ser.serialize_field("candidatesTokenCount", &self.candidates_token_count)?;
        }
        if self.tool_use_prompt_token_count != 0 {
            struct_ser.serialize_field("toolUsePromptTokenCount", &self.tool_use_prompt_token_count)?;
        }
        if self.thoughts_token_count != 0 {
            struct_ser.serialize_field("thoughtsTokenCount", &self.thoughts_token_count)?;
        }
        if self.total_token_count != 0 {
            struct_ser.serialize_field("totalTokenCount", &self.total_token_count)?;
        }
        if !self.prompt_tokens_details.is_empty() {
            struct_ser.serialize_field("promptTokensDetails", &self.prompt_tokens_details)?;
        }
        if !self.cache_tokens_details.is_empty() {
            struct_ser.serialize_field("cacheTokensDetails", &self.cache_tokens_details)?;
        }
        if !self.candidates_tokens_details.is_empty() {
            struct_ser.serialize_field("candidatesTokensDetails", &self.candidates_tokens_details)?;
        }
        if !self.tool_use_prompt_tokens_details.is_empty() {
            struct_ser.serialize_field("toolUsePromptTokensDetails", &self.tool_use_prompt_tokens_details)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for generate_content_response::UsageMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prompt_token_count",
            "promptTokenCount",
            "cached_content_token_count",
            "cachedContentTokenCount",
            "candidates_token_count",
            "candidatesTokenCount",
            "tool_use_prompt_token_count",
            "toolUsePromptTokenCount",
            "thoughts_token_count",
            "thoughtsTokenCount",
            "total_token_count",
            "totalTokenCount",
            "prompt_tokens_details",
            "promptTokensDetails",
            "cache_tokens_details",
            "cacheTokensDetails",
            "candidates_tokens_details",
            "candidatesTokensDetails",
            "tool_use_prompt_tokens_details",
            "toolUsePromptTokensDetails",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PromptTokenCount,
            CachedContentTokenCount,
            CandidatesTokenCount,
            ToolUsePromptTokenCount,
            ThoughtsTokenCount,
            TotalTokenCount,
            PromptTokensDetails,
            CacheTokensDetails,
            CandidatesTokensDetails,
            ToolUsePromptTokensDetails,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "promptTokenCount" | "prompt_token_count" => Ok(GeneratedField::PromptTokenCount),
                            "cachedContentTokenCount" | "cached_content_token_count" => Ok(GeneratedField::CachedContentTokenCount),
                            "candidatesTokenCount" | "candidates_token_count" => Ok(GeneratedField::CandidatesTokenCount),
                            "toolUsePromptTokenCount" | "tool_use_prompt_token_count" => Ok(GeneratedField::ToolUsePromptTokenCount),
                            "thoughtsTokenCount" | "thoughts_token_count" => Ok(GeneratedField::ThoughtsTokenCount),
                            "totalTokenCount" | "total_token_count" => Ok(GeneratedField::TotalTokenCount),
                            "promptTokensDetails" | "prompt_tokens_details" => Ok(GeneratedField::PromptTokensDetails),
                            "cacheTokensDetails" | "cache_tokens_details" => Ok(GeneratedField::CacheTokensDetails),
                            "candidatesTokensDetails" | "candidates_tokens_details" => Ok(GeneratedField::CandidatesTokensDetails),
                            "toolUsePromptTokensDetails" | "tool_use_prompt_tokens_details" => Ok(GeneratedField::ToolUsePromptTokensDetails),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generate_content_response::UsageMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerateContentResponse.UsageMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<generate_content_response::UsageMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prompt_token_count__ = None;
                let mut cached_content_token_count__ = None;
                let mut candidates_token_count__ = None;
                let mut tool_use_prompt_token_count__ = None;
                let mut thoughts_token_count__ = None;
                let mut total_token_count__ = None;
                let mut prompt_tokens_details__ = None;
                let mut cache_tokens_details__ = None;
                let mut candidates_tokens_details__ = None;
                let mut tool_use_prompt_tokens_details__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PromptTokenCount => {
                            if prompt_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptTokenCount"));
                            }
                            prompt_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CachedContentTokenCount => {
                            if cached_content_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cachedContentTokenCount"));
                            }
                            cached_content_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CandidatesTokenCount => {
                            if candidates_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidatesTokenCount"));
                            }
                            candidates_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ToolUsePromptTokenCount => {
                            if tool_use_prompt_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolUsePromptTokenCount"));
                            }
                            tool_use_prompt_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ThoughtsTokenCount => {
                            if thoughts_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thoughtsTokenCount"));
                            }
                            thoughts_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TotalTokenCount => {
                            if total_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalTokenCount"));
                            }
                            total_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PromptTokensDetails => {
                            if prompt_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptTokensDetails"));
                            }
                            prompt_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CacheTokensDetails => {
                            if cache_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cacheTokensDetails"));
                            }
                            cache_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CandidatesTokensDetails => {
                            if candidates_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidatesTokensDetails"));
                            }
                            candidates_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ToolUsePromptTokensDetails => {
                            if tool_use_prompt_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolUsePromptTokensDetails"));
                            }
                            tool_use_prompt_tokens_details__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(generate_content_response::UsageMetadata {
                    prompt_token_count: prompt_token_count__.unwrap_or_default(),
                    cached_content_token_count: cached_content_token_count__.unwrap_or_default(),
                    candidates_token_count: candidates_token_count__.unwrap_or_default(),
                    tool_use_prompt_token_count: tool_use_prompt_token_count__.unwrap_or_default(),
                    thoughts_token_count: thoughts_token_count__.unwrap_or_default(),
                    total_token_count: total_token_count__.unwrap_or_default(),
                    prompt_tokens_details: prompt_tokens_details__.unwrap_or_default(),
                    cache_tokens_details: cache_tokens_details__.unwrap_or_default(),
                    candidates_tokens_details: candidates_tokens_details__.unwrap_or_default(),
                    tool_use_prompt_tokens_details: tool_use_prompt_tokens_details__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerateContentResponse.UsageMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GenerationConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.candidate_count.is_some() {
            len += 1;
        }
        if !self.stop_sequences.is_empty() {
            len += 1;
        }
        if self.max_output_tokens.is_some() {
            len += 1;
        }
        if self.temperature.is_some() {
            len += 1;
        }
        if self.top_p.is_some() {
            len += 1;
        }
        if self.top_k.is_some() {
            len += 1;
        }
        if self.seed.is_some() {
            len += 1;
        }
        if !self.response_mime_type.is_empty() {
            len += 1;
        }
        if self.response_schema.is_some() {
            len += 1;
        }
        if self.response_json_schema.is_some() {
            len += 1;
        }
        if self.response_json_schema_ordered.is_some() {
            len += 1;
        }
        if self.presence_penalty.is_some() {
            len += 1;
        }
        if self.frequency_penalty.is_some() {
            len += 1;
        }
        if self.response_logprobs.is_some() {
            len += 1;
        }
        if self.logprobs.is_some() {
            len += 1;
        }
        if self.enable_enhanced_civic_answers.is_some() {
            len += 1;
        }
        if !self.response_modalities.is_empty() {
            len += 1;
        }
        if self.speech_config.is_some() {
            len += 1;
        }
        if self.thinking_config.is_some() {
            len += 1;
        }
        if self.image_config.is_some() {
            len += 1;
        }
        if self.media_resolution.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GenerationConfig", len)?;
        if let Some(v) = self.candidate_count.as_ref() {
            struct_ser.serialize_field("candidateCount", v)?;
        }
        if !self.stop_sequences.is_empty() {
            struct_ser.serialize_field("stopSequences", &self.stop_sequences)?;
        }
        if let Some(v) = self.max_output_tokens.as_ref() {
            struct_ser.serialize_field("maxOutputTokens", v)?;
        }
        if let Some(v) = self.temperature.as_ref() {
            struct_ser.serialize_field("temperature", v)?;
        }
        if let Some(v) = self.top_p.as_ref() {
            struct_ser.serialize_field("topP", v)?;
        }
        if let Some(v) = self.top_k.as_ref() {
            struct_ser.serialize_field("topK", v)?;
        }
        if let Some(v) = self.seed.as_ref() {
            struct_ser.serialize_field("seed", v)?;
        }
        if !self.response_mime_type.is_empty() {
            struct_ser.serialize_field("responseMimeType", &self.response_mime_type)?;
        }
        if let Some(v) = self.response_schema.as_ref() {
            struct_ser.serialize_field("responseSchema", v)?;
        }
        if let Some(v) = self.response_json_schema.as_ref() {
            struct_ser.serialize_field("_responseJsonSchema", v)?;
        }
        if let Some(v) = self.response_json_schema_ordered.as_ref() {
            struct_ser.serialize_field("responseJsonSchema", v)?;
        }
        if let Some(v) = self.presence_penalty.as_ref() {
            struct_ser.serialize_field("presencePenalty", v)?;
        }
        if let Some(v) = self.frequency_penalty.as_ref() {
            struct_ser.serialize_field("frequencyPenalty", v)?;
        }
        if let Some(v) = self.response_logprobs.as_ref() {
            struct_ser.serialize_field("responseLogprobs", v)?;
        }
        if let Some(v) = self.logprobs.as_ref() {
            struct_ser.serialize_field("logprobs", v)?;
        }
        if let Some(v) = self.enable_enhanced_civic_answers.as_ref() {
            struct_ser.serialize_field("enableEnhancedCivicAnswers", v)?;
        }
        if !self.response_modalities.is_empty() {
            let v = self.response_modalities.iter().cloned().map(|v| {
                generation_config::Modality::try_from(v)
                    .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<std::result::Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("responseModalities", &v)?;
        }
        if let Some(v) = self.speech_config.as_ref() {
            struct_ser.serialize_field("speechConfig", v)?;
        }
        if let Some(v) = self.thinking_config.as_ref() {
            struct_ser.serialize_field("thinkingConfig", v)?;
        }
        if let Some(v) = self.image_config.as_ref() {
            struct_ser.serialize_field("imageConfig", v)?;
        }
        if let Some(v) = self.media_resolution.as_ref() {
            let v = generation_config::MediaResolution::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("mediaResolution", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GenerationConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "candidate_count",
            "candidateCount",
            "stop_sequences",
            "stopSequences",
            "max_output_tokens",
            "maxOutputTokens",
            "temperature",
            "top_p",
            "topP",
            "top_k",
            "topK",
            "seed",
            "response_mime_type",
            "responseMimeType",
            "response_schema",
            "responseSchema",
            "response_json_schema",
            "_responseJsonSchema",
            "response_json_schema_ordered",
            "responseJsonSchema",
            "presence_penalty",
            "presencePenalty",
            "frequency_penalty",
            "frequencyPenalty",
            "response_logprobs",
            "responseLogprobs",
            "logprobs",
            "enable_enhanced_civic_answers",
            "enableEnhancedCivicAnswers",
            "response_modalities",
            "responseModalities",
            "speech_config",
            "speechConfig",
            "thinking_config",
            "thinkingConfig",
            "image_config",
            "imageConfig",
            "media_resolution",
            "mediaResolution",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CandidateCount,
            StopSequences,
            MaxOutputTokens,
            Temperature,
            TopP,
            TopK,
            Seed,
            ResponseMimeType,
            ResponseSchema,
            ResponseJsonSchema,
            ResponseJsonSchemaOrdered,
            PresencePenalty,
            FrequencyPenalty,
            ResponseLogprobs,
            Logprobs,
            EnableEnhancedCivicAnswers,
            ResponseModalities,
            SpeechConfig,
            ThinkingConfig,
            ImageConfig,
            MediaResolution,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "candidateCount" | "candidate_count" => Ok(GeneratedField::CandidateCount),
                            "stopSequences" | "stop_sequences" => Ok(GeneratedField::StopSequences),
                            "maxOutputTokens" | "max_output_tokens" => Ok(GeneratedField::MaxOutputTokens),
                            "temperature" => Ok(GeneratedField::Temperature),
                            "topP" | "top_p" => Ok(GeneratedField::TopP),
                            "topK" | "top_k" => Ok(GeneratedField::TopK),
                            "seed" => Ok(GeneratedField::Seed),
                            "responseMimeType" | "response_mime_type" => Ok(GeneratedField::ResponseMimeType),
                            "responseSchema" | "response_schema" => Ok(GeneratedField::ResponseSchema),
                            "_responseJsonSchema" | "response_json_schema" => Ok(GeneratedField::ResponseJsonSchema),
                            "responseJsonSchema" | "response_json_schema_ordered" => Ok(GeneratedField::ResponseJsonSchemaOrdered),
                            "presencePenalty" | "presence_penalty" => Ok(GeneratedField::PresencePenalty),
                            "frequencyPenalty" | "frequency_penalty" => Ok(GeneratedField::FrequencyPenalty),
                            "responseLogprobs" | "response_logprobs" => Ok(GeneratedField::ResponseLogprobs),
                            "logprobs" => Ok(GeneratedField::Logprobs),
                            "enableEnhancedCivicAnswers" | "enable_enhanced_civic_answers" => Ok(GeneratedField::EnableEnhancedCivicAnswers),
                            "responseModalities" | "response_modalities" => Ok(GeneratedField::ResponseModalities),
                            "speechConfig" | "speech_config" => Ok(GeneratedField::SpeechConfig),
                            "thinkingConfig" | "thinking_config" => Ok(GeneratedField::ThinkingConfig),
                            "imageConfig" | "image_config" => Ok(GeneratedField::ImageConfig),
                            "mediaResolution" | "media_resolution" => Ok(GeneratedField::MediaResolution),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GenerationConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GenerationConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GenerationConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut candidate_count__ = None;
                let mut stop_sequences__ = None;
                let mut max_output_tokens__ = None;
                let mut temperature__ = None;
                let mut top_p__ = None;
                let mut top_k__ = None;
                let mut seed__ = None;
                let mut response_mime_type__ = None;
                let mut response_schema__ = None;
                let mut response_json_schema__ = None;
                let mut response_json_schema_ordered__ = None;
                let mut presence_penalty__ = None;
                let mut frequency_penalty__ = None;
                let mut response_logprobs__ = None;
                let mut logprobs__ = None;
                let mut enable_enhanced_civic_answers__ = None;
                let mut response_modalities__ = None;
                let mut speech_config__ = None;
                let mut thinking_config__ = None;
                let mut image_config__ = None;
                let mut media_resolution__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CandidateCount => {
                            if candidate_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidateCount"));
                            }
                            candidate_count__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::StopSequences => {
                            if stop_sequences__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stopSequences"));
                            }
                            stop_sequences__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MaxOutputTokens => {
                            if max_output_tokens__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxOutputTokens"));
                            }
                            max_output_tokens__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Temperature => {
                            if temperature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("temperature"));
                            }
                            temperature__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TopP => {
                            if top_p__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topP"));
                            }
                            top_p__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TopK => {
                            if top_k__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topK"));
                            }
                            top_k__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Seed => {
                            if seed__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seed"));
                            }
                            seed__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ResponseMimeType => {
                            if response_mime_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseMimeType"));
                            }
                            response_mime_type__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ResponseSchema => {
                            if response_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseSchema"));
                            }
                            response_schema__ = map_.next_value()?;
                        }
                        GeneratedField::ResponseJsonSchema => {
                            if response_json_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("_responseJsonSchema"));
                            }
                            response_json_schema__ = map_.next_value()?;
                        }
                        GeneratedField::ResponseJsonSchemaOrdered => {
                            if response_json_schema_ordered__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseJsonSchema"));
                            }
                            response_json_schema_ordered__ = map_.next_value()?;
                        }
                        GeneratedField::PresencePenalty => {
                            if presence_penalty__.is_some() {
                                return Err(serde::de::Error::duplicate_field("presencePenalty"));
                            }
                            presence_penalty__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::FrequencyPenalty => {
                            if frequency_penalty__.is_some() {
                                return Err(serde::de::Error::duplicate_field("frequencyPenalty"));
                            }
                            frequency_penalty__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::ResponseLogprobs => {
                            if response_logprobs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseLogprobs"));
                            }
                            response_logprobs__ = map_.next_value()?;
                        }
                        GeneratedField::Logprobs => {
                            if logprobs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logprobs"));
                            }
                            logprobs__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::EnableEnhancedCivicAnswers => {
                            if enable_enhanced_civic_answers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enableEnhancedCivicAnswers"));
                            }
                            enable_enhanced_civic_answers__ = map_.next_value()?;
                        }
                        GeneratedField::ResponseModalities => {
                            if response_modalities__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseModalities"));
                            }
                            response_modalities__ = Some(map_.next_value::<Vec<generation_config::Modality>>()?.into_iter().map(|x| x as i32).collect());
                        }
                        GeneratedField::SpeechConfig => {
                            if speech_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("speechConfig"));
                            }
                            speech_config__ = map_.next_value()?;
                        }
                        GeneratedField::ThinkingConfig => {
                            if thinking_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thinkingConfig"));
                            }
                            thinking_config__ = map_.next_value()?;
                        }
                        GeneratedField::ImageConfig => {
                            if image_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("imageConfig"));
                            }
                            image_config__ = map_.next_value()?;
                        }
                        GeneratedField::MediaResolution => {
                            if media_resolution__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mediaResolution"));
                            }
                            media_resolution__ = map_.next_value::<::std::option::Option<generation_config::MediaResolution>>()?.map(|x| x as i32);
                        }
                    }
                }
                Ok(GenerationConfig {
                    candidate_count: candidate_count__,
                    stop_sequences: stop_sequences__.unwrap_or_default(),
                    max_output_tokens: max_output_tokens__,
                    temperature: temperature__,
                    top_p: top_p__,
                    top_k: top_k__,
                    seed: seed__,
                    response_mime_type: response_mime_type__.unwrap_or_default(),
                    response_schema: response_schema__,
                    response_json_schema: response_json_schema__,
                    response_json_schema_ordered: response_json_schema_ordered__,
                    presence_penalty: presence_penalty__,
                    frequency_penalty: frequency_penalty__,
                    response_logprobs: response_logprobs__,
                    logprobs: logprobs__,
                    enable_enhanced_civic_answers: enable_enhanced_civic_answers__,
                    response_modalities: response_modalities__.unwrap_or_default(),
                    speech_config: speech_config__,
                    thinking_config: thinking_config__,
                    image_config: image_config__,
                    media_resolution: media_resolution__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GenerationConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for generation_config::MediaResolution {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MEDIA_RESOLUTION_UNSPECIFIED",
            Self::Low => "MEDIA_RESOLUTION_LOW",
            Self::Medium => "MEDIA_RESOLUTION_MEDIUM",
            Self::High => "MEDIA_RESOLUTION_HIGH",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for generation_config::MediaResolution {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MEDIA_RESOLUTION_UNSPECIFIED",
            "MEDIA_RESOLUTION_LOW",
            "MEDIA_RESOLUTION_MEDIUM",
            "MEDIA_RESOLUTION_HIGH",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generation_config::MediaResolution;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MEDIA_RESOLUTION_UNSPECIFIED" => Ok(generation_config::MediaResolution::Unspecified),
                    "MEDIA_RESOLUTION_LOW" => Ok(generation_config::MediaResolution::Low),
                    "MEDIA_RESOLUTION_MEDIUM" => Ok(generation_config::MediaResolution::Medium),
                    "MEDIA_RESOLUTION_HIGH" => Ok(generation_config::MediaResolution::High),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for generation_config::Modality {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MODALITY_UNSPECIFIED",
            Self::Text => "TEXT",
            Self::Image => "IMAGE",
            Self::Audio => "AUDIO",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for generation_config::Modality {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MODALITY_UNSPECIFIED",
            "TEXT",
            "IMAGE",
            "AUDIO",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = generation_config::Modality;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MODALITY_UNSPECIFIED" => Ok(generation_config::Modality::Unspecified),
                    "TEXT" => Ok(generation_config::Modality::Text),
                    "IMAGE" => Ok(generation_config::Modality::Image),
                    "AUDIO" => Ok(generation_config::Modality::Audio),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for GoAway {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.time_left.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GoAway", len)?;
        if let Some(v) = self.time_left.as_ref() {
            struct_ser.serialize_field("timeLeft", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GoAway {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "time_left",
            "timeLeft",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TimeLeft,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timeLeft" | "time_left" => Ok(GeneratedField::TimeLeft),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GoAway;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GoAway")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GoAway, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut time_left__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TimeLeft => {
                            if time_left__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeLeft"));
                            }
                            time_left__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GoAway {
                    time_left: time_left__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GoAway", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GoogleMaps {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.enable_widget {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GoogleMaps", len)?;
        if self.enable_widget {
            struct_ser.serialize_field("enableWidget", &self.enable_widget)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GoogleMaps {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "enable_widget",
            "enableWidget",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            EnableWidget,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "enableWidget" | "enable_widget" => Ok(GeneratedField::EnableWidget),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GoogleMaps;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GoogleMaps")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GoogleMaps, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut enable_widget__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::EnableWidget => {
                            if enable_widget__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enableWidget"));
                            }
                            enable_widget__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GoogleMaps {
                    enable_widget: enable_widget__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GoogleMaps", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GoogleSearchRetrieval {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.dynamic_retrieval_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GoogleSearchRetrieval", len)?;
        if let Some(v) = self.dynamic_retrieval_config.as_ref() {
            struct_ser.serialize_field("dynamicRetrievalConfig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GoogleSearchRetrieval {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dynamic_retrieval_config",
            "dynamicRetrievalConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DynamicRetrievalConfig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "dynamicRetrievalConfig" | "dynamic_retrieval_config" => Ok(GeneratedField::DynamicRetrievalConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GoogleSearchRetrieval;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GoogleSearchRetrieval")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GoogleSearchRetrieval, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dynamic_retrieval_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DynamicRetrievalConfig => {
                            if dynamic_retrieval_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dynamicRetrievalConfig"));
                            }
                            dynamic_retrieval_config__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GoogleSearchRetrieval {
                    dynamic_retrieval_config: dynamic_retrieval_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GoogleSearchRetrieval", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingAttribution {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.source_id.is_some() {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingAttribution", len)?;
        if let Some(v) = self.source_id.as_ref() {
            struct_ser.serialize_field("sourceId", v)?;
        }
        if let Some(v) = self.content.as_ref() {
            struct_ser.serialize_field("content", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingAttribution {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "source_id",
            "sourceId",
            "content",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SourceId,
            Content,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "sourceId" | "source_id" => Ok(GeneratedField::SourceId),
                            "content" => Ok(GeneratedField::Content),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingAttribution;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingAttribution")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingAttribution, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut source_id__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SourceId => {
                            if source_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceId"));
                            }
                            source_id__ = map_.next_value()?;
                        }
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GroundingAttribution {
                    source_id: source_id__,
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingAttribution", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingChunk {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.chunk_type.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk", len)?;
        if let Some(v) = self.chunk_type.as_ref() {
            match v {
                grounding_chunk::ChunkType::Web(v) => {
                    struct_ser.serialize_field("web", v)?;
                }
                grounding_chunk::ChunkType::RetrievedContext(v) => {
                    struct_ser.serialize_field("retrievedContext", v)?;
                }
                grounding_chunk::ChunkType::Maps(v) => {
                    struct_ser.serialize_field("maps", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingChunk {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "web",
            "retrieved_context",
            "retrievedContext",
            "maps",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Web,
            RetrievedContext,
            Maps,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "web" => Ok(GeneratedField::Web),
                            "retrievedContext" | "retrieved_context" => Ok(GeneratedField::RetrievedContext),
                            "maps" => Ok(GeneratedField::Maps),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingChunk;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingChunk, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut chunk_type__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Web => {
                            if chunk_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("web"));
                            }
                            chunk_type__ = map_.next_value::<::std::option::Option<_>>()?.map(grounding_chunk::ChunkType::Web)
;
                        }
                        GeneratedField::RetrievedContext => {
                            if chunk_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievedContext"));
                            }
                            chunk_type__ = map_.next_value::<::std::option::Option<_>>()?.map(grounding_chunk::ChunkType::RetrievedContext)
;
                        }
                        GeneratedField::Maps => {
                            if chunk_type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maps"));
                            }
                            chunk_type__ = map_.next_value::<::std::option::Option<_>>()?.map(grounding_chunk::ChunkType::Maps)
;
                        }
                    }
                }
                Ok(GroundingChunk {
                    chunk_type: chunk_type__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for grounding_chunk::Maps {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.uri.is_some() {
            len += 1;
        }
        if self.title.is_some() {
            len += 1;
        }
        if self.text.is_some() {
            len += 1;
        }
        if self.place_id.is_some() {
            len += 1;
        }
        if self.place_answer_sources.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps", len)?;
        if let Some(v) = self.uri.as_ref() {
            struct_ser.serialize_field("uri", v)?;
        }
        if let Some(v) = self.title.as_ref() {
            struct_ser.serialize_field("title", v)?;
        }
        if let Some(v) = self.text.as_ref() {
            struct_ser.serialize_field("text", v)?;
        }
        if let Some(v) = self.place_id.as_ref() {
            struct_ser.serialize_field("placeId", v)?;
        }
        if let Some(v) = self.place_answer_sources.as_ref() {
            struct_ser.serialize_field("placeAnswerSources", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for grounding_chunk::Maps {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "uri",
            "title",
            "text",
            "place_id",
            "placeId",
            "place_answer_sources",
            "placeAnswerSources",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Uri,
            Title,
            Text,
            PlaceId,
            PlaceAnswerSources,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "uri" => Ok(GeneratedField::Uri),
                            "title" => Ok(GeneratedField::Title),
                            "text" => Ok(GeneratedField::Text),
                            "placeId" | "place_id" => Ok(GeneratedField::PlaceId),
                            "placeAnswerSources" | "place_answer_sources" => Ok(GeneratedField::PlaceAnswerSources),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = grounding_chunk::Maps;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk.Maps")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<grounding_chunk::Maps, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut uri__ = None;
                let mut title__ = None;
                let mut text__ = None;
                let mut place_id__ = None;
                let mut place_answer_sources__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Uri => {
                            if uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uri"));
                            }
                            uri__ = map_.next_value()?;
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = map_.next_value()?;
                        }
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = map_.next_value()?;
                        }
                        GeneratedField::PlaceId => {
                            if place_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("placeId"));
                            }
                            place_id__ = map_.next_value()?;
                        }
                        GeneratedField::PlaceAnswerSources => {
                            if place_answer_sources__.is_some() {
                                return Err(serde::de::Error::duplicate_field("placeAnswerSources"));
                            }
                            place_answer_sources__ = map_.next_value()?;
                        }
                    }
                }
                Ok(grounding_chunk::Maps {
                    uri: uri__,
                    title: title__,
                    text: text__,
                    place_id: place_id__,
                    place_answer_sources: place_answer_sources__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for grounding_chunk::maps::PlaceAnswerSources {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.review_snippets.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources", len)?;
        if !self.review_snippets.is_empty() {
            struct_ser.serialize_field("reviewSnippets", &self.review_snippets)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for grounding_chunk::maps::PlaceAnswerSources {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "review_snippets",
            "reviewSnippets",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReviewSnippets,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "reviewSnippets" | "review_snippets" => Ok(GeneratedField::ReviewSnippets),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = grounding_chunk::maps::PlaceAnswerSources;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<grounding_chunk::maps::PlaceAnswerSources, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut review_snippets__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ReviewSnippets => {
                            if review_snippets__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reviewSnippets"));
                            }
                            review_snippets__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(grounding_chunk::maps::PlaceAnswerSources {
                    review_snippets: review_snippets__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for grounding_chunk::maps::place_answer_sources::ReviewSnippet {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.review_id.is_some() {
            len += 1;
        }
        if self.google_maps_uri.is_some() {
            len += 1;
        }
        if self.title.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources.ReviewSnippet", len)?;
        if let Some(v) = self.review_id.as_ref() {
            struct_ser.serialize_field("reviewId", v)?;
        }
        if let Some(v) = self.google_maps_uri.as_ref() {
            struct_ser.serialize_field("googleMapsUri", v)?;
        }
        if let Some(v) = self.title.as_ref() {
            struct_ser.serialize_field("title", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for grounding_chunk::maps::place_answer_sources::ReviewSnippet {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "review_id",
            "reviewId",
            "google_maps_uri",
            "googleMapsUri",
            "title",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ReviewId,
            GoogleMapsUri,
            Title,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "reviewId" | "review_id" => Ok(GeneratedField::ReviewId),
                            "googleMapsUri" | "google_maps_uri" => Ok(GeneratedField::GoogleMapsUri),
                            "title" => Ok(GeneratedField::Title),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = grounding_chunk::maps::place_answer_sources::ReviewSnippet;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources.ReviewSnippet")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<grounding_chunk::maps::place_answer_sources::ReviewSnippet, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut review_id__ = None;
                let mut google_maps_uri__ = None;
                let mut title__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ReviewId => {
                            if review_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reviewId"));
                            }
                            review_id__ = map_.next_value()?;
                        }
                        GeneratedField::GoogleMapsUri => {
                            if google_maps_uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleMapsUri"));
                            }
                            google_maps_uri__ = map_.next_value()?;
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = map_.next_value()?;
                        }
                    }
                }
                Ok(grounding_chunk::maps::place_answer_sources::ReviewSnippet {
                    review_id: review_id__,
                    google_maps_uri: google_maps_uri__,
                    title: title__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Maps.PlaceAnswerSources.ReviewSnippet", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for grounding_chunk::RetrievedContext {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.uri.is_some() {
            len += 1;
        }
        if self.title.is_some() {
            len += 1;
        }
        if self.text.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.RetrievedContext", len)?;
        if let Some(v) = self.uri.as_ref() {
            struct_ser.serialize_field("uri", v)?;
        }
        if let Some(v) = self.title.as_ref() {
            struct_ser.serialize_field("title", v)?;
        }
        if let Some(v) = self.text.as_ref() {
            struct_ser.serialize_field("text", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for grounding_chunk::RetrievedContext {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "uri",
            "title",
            "text",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Uri,
            Title,
            Text,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "uri" => Ok(GeneratedField::Uri),
                            "title" => Ok(GeneratedField::Title),
                            "text" => Ok(GeneratedField::Text),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = grounding_chunk::RetrievedContext;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk.RetrievedContext")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<grounding_chunk::RetrievedContext, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut uri__ = None;
                let mut title__ = None;
                let mut text__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Uri => {
                            if uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uri"));
                            }
                            uri__ = map_.next_value()?;
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = map_.next_value()?;
                        }
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = map_.next_value()?;
                        }
                    }
                }
                Ok(grounding_chunk::RetrievedContext {
                    uri: uri__,
                    title: title__,
                    text: text__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.RetrievedContext", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for grounding_chunk::Web {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.uri.is_some() {
            len += 1;
        }
        if self.title.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Web", len)?;
        if let Some(v) = self.uri.as_ref() {
            struct_ser.serialize_field("uri", v)?;
        }
        if let Some(v) = self.title.as_ref() {
            struct_ser.serialize_field("title", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for grounding_chunk::Web {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "uri",
            "title",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Uri,
            Title,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "uri" => Ok(GeneratedField::Uri),
                            "title" => Ok(GeneratedField::Title),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = grounding_chunk::Web;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingChunk.Web")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<grounding_chunk::Web, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut uri__ = None;
                let mut title__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Uri => {
                            if uri__.is_some() {
                                return Err(serde::de::Error::duplicate_field("uri"));
                            }
                            uri__ = map_.next_value()?;
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = map_.next_value()?;
                        }
                    }
                }
                Ok(grounding_chunk::Web {
                    uri: uri__,
                    title: title__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingChunk.Web", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.search_entry_point.is_some() {
            len += 1;
        }
        if !self.grounding_chunks.is_empty() {
            len += 1;
        }
        if !self.grounding_supports.is_empty() {
            len += 1;
        }
        if self.retrieval_metadata.is_some() {
            len += 1;
        }
        if !self.web_search_queries.is_empty() {
            len += 1;
        }
        if self.google_maps_widget_context_token.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingMetadata", len)?;
        if let Some(v) = self.search_entry_point.as_ref() {
            struct_ser.serialize_field("searchEntryPoint", v)?;
        }
        if !self.grounding_chunks.is_empty() {
            struct_ser.serialize_field("groundingChunks", &self.grounding_chunks)?;
        }
        if !self.grounding_supports.is_empty() {
            struct_ser.serialize_field("groundingSupports", &self.grounding_supports)?;
        }
        if let Some(v) = self.retrieval_metadata.as_ref() {
            struct_ser.serialize_field("retrievalMetadata", v)?;
        }
        if !self.web_search_queries.is_empty() {
            struct_ser.serialize_field("webSearchQueries", &self.web_search_queries)?;
        }
        if let Some(v) = self.google_maps_widget_context_token.as_ref() {
            struct_ser.serialize_field("googleMapsWidgetContextToken", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "search_entry_point",
            "searchEntryPoint",
            "grounding_chunks",
            "groundingChunks",
            "grounding_supports",
            "groundingSupports",
            "retrieval_metadata",
            "retrievalMetadata",
            "web_search_queries",
            "webSearchQueries",
            "google_maps_widget_context_token",
            "googleMapsWidgetContextToken",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SearchEntryPoint,
            GroundingChunks,
            GroundingSupports,
            RetrievalMetadata,
            WebSearchQueries,
            GoogleMapsWidgetContextToken,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "searchEntryPoint" | "search_entry_point" => Ok(GeneratedField::SearchEntryPoint),
                            "groundingChunks" | "grounding_chunks" => Ok(GeneratedField::GroundingChunks),
                            "groundingSupports" | "grounding_supports" => Ok(GeneratedField::GroundingSupports),
                            "retrievalMetadata" | "retrieval_metadata" => Ok(GeneratedField::RetrievalMetadata),
                            "webSearchQueries" | "web_search_queries" => Ok(GeneratedField::WebSearchQueries),
                            "googleMapsWidgetContextToken" | "google_maps_widget_context_token" => Ok(GeneratedField::GoogleMapsWidgetContextToken),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut search_entry_point__ = None;
                let mut grounding_chunks__ = None;
                let mut grounding_supports__ = None;
                let mut retrieval_metadata__ = None;
                let mut web_search_queries__ = None;
                let mut google_maps_widget_context_token__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SearchEntryPoint => {
                            if search_entry_point__.is_some() {
                                return Err(serde::de::Error::duplicate_field("searchEntryPoint"));
                            }
                            search_entry_point__ = map_.next_value()?;
                        }
                        GeneratedField::GroundingChunks => {
                            if grounding_chunks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingChunks"));
                            }
                            grounding_chunks__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GroundingSupports => {
                            if grounding_supports__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingSupports"));
                            }
                            grounding_supports__ = Some(map_.next_value()?);
                        }
                        GeneratedField::RetrievalMetadata => {
                            if retrieval_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievalMetadata"));
                            }
                            retrieval_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::WebSearchQueries => {
                            if web_search_queries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("webSearchQueries"));
                            }
                            web_search_queries__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GoogleMapsWidgetContextToken => {
                            if google_maps_widget_context_token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleMapsWidgetContextToken"));
                            }
                            google_maps_widget_context_token__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GroundingMetadata {
                    search_entry_point: search_entry_point__,
                    grounding_chunks: grounding_chunks__.unwrap_or_default(),
                    grounding_supports: grounding_supports__.unwrap_or_default(),
                    retrieval_metadata: retrieval_metadata__,
                    web_search_queries: web_search_queries__.unwrap_or_default(),
                    google_maps_widget_context_token: google_maps_widget_context_token__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingPassage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingPassage", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.content.as_ref() {
            struct_ser.serialize_field("content", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingPassage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "content",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Content,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "content" => Ok(GeneratedField::Content),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingPassage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingPassage")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingPassage, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = map_.next_value()?;
                        }
                    }
                }
                Ok(GroundingPassage {
                    id: id__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingPassage", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingPassages {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.passages.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingPassages", len)?;
        if !self.passages.is_empty() {
            struct_ser.serialize_field("passages", &self.passages)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingPassages {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "passages",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Passages,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "passages" => Ok(GeneratedField::Passages),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingPassages;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingPassages")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingPassages, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut passages__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Passages => {
                            if passages__.is_some() {
                                return Err(serde::de::Error::duplicate_field("passages"));
                            }
                            passages__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GroundingPassages {
                    passages: passages__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingPassages", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GroundingSupport {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.segment.is_some() {
            len += 1;
        }
        if !self.grounding_chunk_indices.is_empty() {
            len += 1;
        }
        if !self.confidence_scores.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.GroundingSupport", len)?;
        if let Some(v) = self.segment.as_ref() {
            struct_ser.serialize_field("segment", v)?;
        }
        if !self.grounding_chunk_indices.is_empty() {
            struct_ser.serialize_field("groundingChunkIndices", &self.grounding_chunk_indices)?;
        }
        if !self.confidence_scores.is_empty() {
            struct_ser.serialize_field("confidenceScores", &self.confidence_scores)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GroundingSupport {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "segment",
            "grounding_chunk_indices",
            "groundingChunkIndices",
            "confidence_scores",
            "confidenceScores",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Segment,
            GroundingChunkIndices,
            ConfidenceScores,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "segment" => Ok(GeneratedField::Segment),
                            "groundingChunkIndices" | "grounding_chunk_indices" => Ok(GeneratedField::GroundingChunkIndices),
                            "confidenceScores" | "confidence_scores" => Ok(GeneratedField::ConfidenceScores),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GroundingSupport;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.GroundingSupport")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GroundingSupport, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut segment__ = None;
                let mut grounding_chunk_indices__ = None;
                let mut confidence_scores__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Segment => {
                            if segment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("segment"));
                            }
                            segment__ = map_.next_value()?;
                        }
                        GeneratedField::GroundingChunkIndices => {
                            if grounding_chunk_indices__.is_some() {
                                return Err(serde::de::Error::duplicate_field("groundingChunkIndices"));
                            }
                            grounding_chunk_indices__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                        GeneratedField::ConfidenceScores => {
                            if confidence_scores__.is_some() {
                                return Err(serde::de::Error::duplicate_field("confidenceScores"));
                            }
                            confidence_scores__ = 
                                Some(map_.next_value::<Vec<::pbjson::private::NumberDeserialize<_>>>()?
                                    .into_iter().map(|x| x.0).collect())
                            ;
                        }
                    }
                }
                Ok(GroundingSupport {
                    segment: segment__,
                    grounding_chunk_indices: grounding_chunk_indices__.unwrap_or_default(),
                    confidence_scores: confidence_scores__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.GroundingSupport", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HarmCategory {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "HARM_CATEGORY_UNSPECIFIED",
            Self::Derogatory => "HARM_CATEGORY_DEROGATORY",
            Self::Toxicity => "HARM_CATEGORY_TOXICITY",
            Self::Violence => "HARM_CATEGORY_VIOLENCE",
            Self::Sexual => "HARM_CATEGORY_SEXUAL",
            Self::Medical => "HARM_CATEGORY_MEDICAL",
            Self::Dangerous => "HARM_CATEGORY_DANGEROUS",
            Self::Harassment => "HARM_CATEGORY_HARASSMENT",
            Self::HateSpeech => "HARM_CATEGORY_HATE_SPEECH",
            Self::SexuallyExplicit => "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            Self::DangerousContent => "HARM_CATEGORY_DANGEROUS_CONTENT",
            Self::CivicIntegrity => "HARM_CATEGORY_CIVIC_INTEGRITY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for HarmCategory {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "HARM_CATEGORY_UNSPECIFIED",
            "HARM_CATEGORY_DEROGATORY",
            "HARM_CATEGORY_TOXICITY",
            "HARM_CATEGORY_VIOLENCE",
            "HARM_CATEGORY_SEXUAL",
            "HARM_CATEGORY_MEDICAL",
            "HARM_CATEGORY_DANGEROUS",
            "HARM_CATEGORY_HARASSMENT",
            "HARM_CATEGORY_HATE_SPEECH",
            "HARM_CATEGORY_SEXUALLY_EXPLICIT",
            "HARM_CATEGORY_DANGEROUS_CONTENT",
            "HARM_CATEGORY_CIVIC_INTEGRITY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = HarmCategory;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "HARM_CATEGORY_UNSPECIFIED" => Ok(HarmCategory::Unspecified),
                    "HARM_CATEGORY_DEROGATORY" => Ok(HarmCategory::Derogatory),
                    "HARM_CATEGORY_TOXICITY" => Ok(HarmCategory::Toxicity),
                    "HARM_CATEGORY_VIOLENCE" => Ok(HarmCategory::Violence),
                    "HARM_CATEGORY_SEXUAL" => Ok(HarmCategory::Sexual),
                    "HARM_CATEGORY_MEDICAL" => Ok(HarmCategory::Medical),
                    "HARM_CATEGORY_DANGEROUS" => Ok(HarmCategory::Dangerous),
                    "HARM_CATEGORY_HARASSMENT" => Ok(HarmCategory::Harassment),
                    "HARM_CATEGORY_HATE_SPEECH" => Ok(HarmCategory::HateSpeech),
                    "HARM_CATEGORY_SEXUALLY_EXPLICIT" => Ok(HarmCategory::SexuallyExplicit),
                    "HARM_CATEGORY_DANGEROUS_CONTENT" => Ok(HarmCategory::DangerousContent),
                    "HARM_CATEGORY_CIVIC_INTEGRITY" => Ok(HarmCategory::CivicIntegrity),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ImageConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.aspect_ratio.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ImageConfig", len)?;
        if let Some(v) = self.aspect_ratio.as_ref() {
            struct_ser.serialize_field("aspectRatio", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ImageConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "aspect_ratio",
            "aspectRatio",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AspectRatio,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "aspectRatio" | "aspect_ratio" => Ok(GeneratedField::AspectRatio),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ImageConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ImageConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ImageConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut aspect_ratio__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AspectRatio => {
                            if aspect_ratio__.is_some() {
                                return Err(serde::de::Error::duplicate_field("aspectRatio"));
                            }
                            aspect_ratio__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ImageConfig {
                    aspect_ratio: aspect_ratio__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ImageConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LogprobsResult {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.log_probability_sum.is_some() {
            len += 1;
        }
        if !self.top_candidates.is_empty() {
            len += 1;
        }
        if !self.chosen_candidates.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult", len)?;
        if let Some(v) = self.log_probability_sum.as_ref() {
            struct_ser.serialize_field("logProbabilitySum", v)?;
        }
        if !self.top_candidates.is_empty() {
            struct_ser.serialize_field("topCandidates", &self.top_candidates)?;
        }
        if !self.chosen_candidates.is_empty() {
            struct_ser.serialize_field("chosenCandidates", &self.chosen_candidates)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LogprobsResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "log_probability_sum",
            "logProbabilitySum",
            "top_candidates",
            "topCandidates",
            "chosen_candidates",
            "chosenCandidates",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LogProbabilitySum,
            TopCandidates,
            ChosenCandidates,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "logProbabilitySum" | "log_probability_sum" => Ok(GeneratedField::LogProbabilitySum),
                            "topCandidates" | "top_candidates" => Ok(GeneratedField::TopCandidates),
                            "chosenCandidates" | "chosen_candidates" => Ok(GeneratedField::ChosenCandidates),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LogprobsResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.LogprobsResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LogprobsResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut log_probability_sum__ = None;
                let mut top_candidates__ = None;
                let mut chosen_candidates__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::LogProbabilitySum => {
                            if log_probability_sum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logProbabilitySum"));
                            }
                            log_probability_sum__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::TopCandidates => {
                            if top_candidates__.is_some() {
                                return Err(serde::de::Error::duplicate_field("topCandidates"));
                            }
                            top_candidates__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ChosenCandidates => {
                            if chosen_candidates__.is_some() {
                                return Err(serde::de::Error::duplicate_field("chosenCandidates"));
                            }
                            chosen_candidates__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(LogprobsResult {
                    log_probability_sum: log_probability_sum__,
                    top_candidates: top_candidates__.unwrap_or_default(),
                    chosen_candidates: chosen_candidates__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for logprobs_result::Candidate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.token.is_some() {
            len += 1;
        }
        if self.token_id.is_some() {
            len += 1;
        }
        if self.log_probability.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult.Candidate", len)?;
        if let Some(v) = self.token.as_ref() {
            struct_ser.serialize_field("token", v)?;
        }
        if let Some(v) = self.token_id.as_ref() {
            struct_ser.serialize_field("tokenId", v)?;
        }
        if let Some(v) = self.log_probability.as_ref() {
            struct_ser.serialize_field("logProbability", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for logprobs_result::Candidate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "token",
            "token_id",
            "tokenId",
            "log_probability",
            "logProbability",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Token,
            TokenId,
            LogProbability,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "token" => Ok(GeneratedField::Token),
                            "tokenId" | "token_id" => Ok(GeneratedField::TokenId),
                            "logProbability" | "log_probability" => Ok(GeneratedField::LogProbability),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = logprobs_result::Candidate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.LogprobsResult.Candidate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<logprobs_result::Candidate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut token__ = None;
                let mut token_id__ = None;
                let mut log_probability__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Token => {
                            if token__.is_some() {
                                return Err(serde::de::Error::duplicate_field("token"));
                            }
                            token__ = map_.next_value()?;
                        }
                        GeneratedField::TokenId => {
                            if token_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenId"));
                            }
                            token_id__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::LogProbability => {
                            if log_probability__.is_some() {
                                return Err(serde::de::Error::duplicate_field("logProbability"));
                            }
                            log_probability__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(logprobs_result::Candidate {
                    token: token__,
                    token_id: token_id__,
                    log_probability: log_probability__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult.Candidate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for logprobs_result::TopCandidates {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.candidates.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult.TopCandidates", len)?;
        if !self.candidates.is_empty() {
            struct_ser.serialize_field("candidates", &self.candidates)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for logprobs_result::TopCandidates {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "candidates",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Candidates,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "candidates" => Ok(GeneratedField::Candidates),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = logprobs_result::TopCandidates;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.LogprobsResult.TopCandidates")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<logprobs_result::TopCandidates, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut candidates__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Candidates => {
                            if candidates__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidates"));
                            }
                            candidates__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(logprobs_result::TopCandidates {
                    candidates: candidates__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.LogprobsResult.TopCandidates", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MetadataFilter {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.key.is_empty() {
            len += 1;
        }
        if !self.conditions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.MetadataFilter", len)?;
        if !self.key.is_empty() {
            struct_ser.serialize_field("key", &self.key)?;
        }
        if !self.conditions.is_empty() {
            struct_ser.serialize_field("conditions", &self.conditions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MetadataFilter {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "key",
            "conditions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Key,
            Conditions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "key" => Ok(GeneratedField::Key),
                            "conditions" => Ok(GeneratedField::Conditions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MetadataFilter;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.MetadataFilter")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MetadataFilter, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut key__ = None;
                let mut conditions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Key => {
                            if key__.is_some() {
                                return Err(serde::de::Error::duplicate_field("key"));
                            }
                            key__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Conditions => {
                            if conditions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("conditions"));
                            }
                            conditions__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(MetadataFilter {
                    key: key__.unwrap_or_default(),
                    conditions: conditions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.MetadataFilter", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Modality {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MODALITY_UNSPECIFIED",
            Self::Text => "TEXT",
            Self::Image => "IMAGE",
            Self::Video => "VIDEO",
            Self::Audio => "AUDIO",
            Self::Document => "DOCUMENT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for Modality {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MODALITY_UNSPECIFIED",
            "TEXT",
            "IMAGE",
            "VIDEO",
            "AUDIO",
            "DOCUMENT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Modality;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "MODALITY_UNSPECIFIED" => Ok(Modality::Unspecified),
                    "TEXT" => Ok(Modality::Text),
                    "IMAGE" => Ok(Modality::Image),
                    "VIDEO" => Ok(Modality::Video),
                    "AUDIO" => Ok(Modality::Audio),
                    "DOCUMENT" => Ok(Modality::Document),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ModalityTokenCount {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.modality != 0 {
            len += 1;
        }
        if self.token_count != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ModalityTokenCount", len)?;
        if self.modality != 0 {
            let v = Modality::try_from(self.modality)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.modality)))?;
            struct_ser.serialize_field("modality", &v)?;
        }
        if self.token_count != 0 {
            struct_ser.serialize_field("tokenCount", &self.token_count)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModalityTokenCount {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "modality",
            "token_count",
            "tokenCount",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Modality,
            TokenCount,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "modality" => Ok(GeneratedField::Modality),
                            "tokenCount" | "token_count" => Ok(GeneratedField::TokenCount),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModalityTokenCount;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ModalityTokenCount")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ModalityTokenCount, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut modality__ = None;
                let mut token_count__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Modality => {
                            if modality__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modality"));
                            }
                            modality__ = Some(map_.next_value::<Modality>()? as i32);
                        }
                        GeneratedField::TokenCount => {
                            if token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tokenCount"));
                            }
                            token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(ModalityTokenCount {
                    modality: modality__.unwrap_or_default(),
                    token_count: token_count__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ModalityTokenCount", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MultiSpeakerVoiceConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.speaker_voice_configs.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.MultiSpeakerVoiceConfig", len)?;
        if !self.speaker_voice_configs.is_empty() {
            struct_ser.serialize_field("speakerVoiceConfigs", &self.speaker_voice_configs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MultiSpeakerVoiceConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "speaker_voice_configs",
            "speakerVoiceConfigs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SpeakerVoiceConfigs,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "speakerVoiceConfigs" | "speaker_voice_configs" => Ok(GeneratedField::SpeakerVoiceConfigs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MultiSpeakerVoiceConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.MultiSpeakerVoiceConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MultiSpeakerVoiceConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut speaker_voice_configs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SpeakerVoiceConfigs => {
                            if speaker_voice_configs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("speakerVoiceConfigs"));
                            }
                            speaker_voice_configs__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(MultiSpeakerVoiceConfig {
                    speaker_voice_configs: speaker_voice_configs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.MultiSpeakerVoiceConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Part {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.thought {
            len += 1;
        }
        if !self.thought_signature.is_empty() {
            len += 1;
        }
        if self.part_metadata.is_some() {
            len += 1;
        }
        if self.data.is_some() {
            len += 1;
        }
        if self.metadata.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Part", len)?;
        if self.thought {
            struct_ser.serialize_field("thought", &self.thought)?;
        }
        if !self.thought_signature.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("thoughtSignature", pbjson::private::base64::encode(&self.thought_signature).as_str())?;
        }
        if let Some(v) = self.part_metadata.as_ref() {
            struct_ser.serialize_field("partMetadata", v)?;
        }
        if let Some(v) = self.data.as_ref() {
            match v {
                part::Data::Text(v) => {
                    struct_ser.serialize_field("text", v)?;
                }
                part::Data::InlineData(v) => {
                    struct_ser.serialize_field("inlineData", v)?;
                }
                part::Data::FunctionCall(v) => {
                    struct_ser.serialize_field("functionCall", v)?;
                }
                part::Data::FunctionResponse(v) => {
                    struct_ser.serialize_field("functionResponse", v)?;
                }
                part::Data::FileData(v) => {
                    struct_ser.serialize_field("fileData", v)?;
                }
                part::Data::ExecutableCode(v) => {
                    struct_ser.serialize_field("executableCode", v)?;
                }
                part::Data::CodeExecutionResult(v) => {
                    struct_ser.serialize_field("codeExecutionResult", v)?;
                }
            }
        }
        if let Some(v) = self.metadata.as_ref() {
            match v {
                part::Metadata::VideoMetadata(v) => {
                    struct_ser.serialize_field("videoMetadata", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Part {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "thought",
            "thought_signature",
            "thoughtSignature",
            "part_metadata",
            "partMetadata",
            "text",
            "inline_data",
            "inlineData",
            "function_call",
            "functionCall",
            "function_response",
            "functionResponse",
            "file_data",
            "fileData",
            "executable_code",
            "executableCode",
            "code_execution_result",
            "codeExecutionResult",
            "video_metadata",
            "videoMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Thought,
            ThoughtSignature,
            PartMetadata,
            Text,
            InlineData,
            FunctionCall,
            FunctionResponse,
            FileData,
            ExecutableCode,
            CodeExecutionResult,
            VideoMetadata,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "thought" => Ok(GeneratedField::Thought),
                            "thoughtSignature" | "thought_signature" => Ok(GeneratedField::ThoughtSignature),
                            "partMetadata" | "part_metadata" => Ok(GeneratedField::PartMetadata),
                            "text" => Ok(GeneratedField::Text),
                            "inlineData" | "inline_data" => Ok(GeneratedField::InlineData),
                            "functionCall" | "function_call" => Ok(GeneratedField::FunctionCall),
                            "functionResponse" | "function_response" => Ok(GeneratedField::FunctionResponse),
                            "fileData" | "file_data" => Ok(GeneratedField::FileData),
                            "executableCode" | "executable_code" => Ok(GeneratedField::ExecutableCode),
                            "codeExecutionResult" | "code_execution_result" => Ok(GeneratedField::CodeExecutionResult),
                            "videoMetadata" | "video_metadata" => Ok(GeneratedField::VideoMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Part;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Part")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Part, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut thought__ = None;
                let mut thought_signature__ = None;
                let mut part_metadata__ = None;
                let mut data__ = None;
                let mut metadata__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Thought => {
                            if thought__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thought"));
                            }
                            thought__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ThoughtSignature => {
                            if thought_signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thoughtSignature"));
                            }
                            thought_signature__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PartMetadata => {
                            if part_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("partMetadata"));
                            }
                            part_metadata__ = map_.next_value()?;
                        }
                        GeneratedField::Text => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::Text);
                        }
                        GeneratedField::InlineData => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inlineData"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::InlineData)
;
                        }
                        GeneratedField::FunctionCall => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionCall"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::FunctionCall)
;
                        }
                        GeneratedField::FunctionResponse => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionResponse"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::FunctionResponse)
;
                        }
                        GeneratedField::FileData => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fileData"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::FileData)
;
                        }
                        GeneratedField::ExecutableCode => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("executableCode"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::ExecutableCode)
;
                        }
                        GeneratedField::CodeExecutionResult => {
                            if data__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codeExecutionResult"));
                            }
                            data__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Data::CodeExecutionResult)
;
                        }
                        GeneratedField::VideoMetadata => {
                            if metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("videoMetadata"));
                            }
                            metadata__ = map_.next_value::<::std::option::Option<_>>()?.map(part::Metadata::VideoMetadata)
;
                        }
                    }
                }
                Ok(Part {
                    thought: thought__.unwrap_or_default(),
                    thought_signature: thought_signature__.unwrap_or_default(),
                    part_metadata: part_metadata__,
                    data: data__,
                    metadata: metadata__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Part", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PrebuiltVoiceConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.voice_name.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.PrebuiltVoiceConfig", len)?;
        if let Some(v) = self.voice_name.as_ref() {
            struct_ser.serialize_field("voiceName", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PrebuiltVoiceConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "voice_name",
            "voiceName",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            VoiceName,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "voiceName" | "voice_name" => Ok(GeneratedField::VoiceName),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PrebuiltVoiceConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.PrebuiltVoiceConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PrebuiltVoiceConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut voice_name__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::VoiceName => {
                            if voice_name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voiceName"));
                            }
                            voice_name__ = map_.next_value()?;
                        }
                    }
                }
                Ok(PrebuiltVoiceConfig {
                    voice_name: voice_name__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.PrebuiltVoiceConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RealtimeInputConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.automatic_activity_detection.is_some() {
            len += 1;
        }
        if self.activity_handling.is_some() {
            len += 1;
        }
        if self.turn_coverage.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.RealtimeInputConfig", len)?;
        if let Some(v) = self.automatic_activity_detection.as_ref() {
            struct_ser.serialize_field("automaticActivityDetection", v)?;
        }
        if let Some(v) = self.activity_handling.as_ref() {
            let v = realtime_input_config::ActivityHandling::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("activityHandling", &v)?;
        }
        if let Some(v) = self.turn_coverage.as_ref() {
            let v = realtime_input_config::TurnCoverage::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("turnCoverage", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RealtimeInputConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "automatic_activity_detection",
            "automaticActivityDetection",
            "activity_handling",
            "activityHandling",
            "turn_coverage",
            "turnCoverage",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AutomaticActivityDetection,
            ActivityHandling,
            TurnCoverage,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "automaticActivityDetection" | "automatic_activity_detection" => Ok(GeneratedField::AutomaticActivityDetection),
                            "activityHandling" | "activity_handling" => Ok(GeneratedField::ActivityHandling),
                            "turnCoverage" | "turn_coverage" => Ok(GeneratedField::TurnCoverage),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RealtimeInputConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.RealtimeInputConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RealtimeInputConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut automatic_activity_detection__ = None;
                let mut activity_handling__ = None;
                let mut turn_coverage__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AutomaticActivityDetection => {
                            if automatic_activity_detection__.is_some() {
                                return Err(serde::de::Error::duplicate_field("automaticActivityDetection"));
                            }
                            automatic_activity_detection__ = map_.next_value()?;
                        }
                        GeneratedField::ActivityHandling => {
                            if activity_handling__.is_some() {
                                return Err(serde::de::Error::duplicate_field("activityHandling"));
                            }
                            activity_handling__ = map_.next_value::<::std::option::Option<realtime_input_config::ActivityHandling>>()?.map(|x| x as i32);
                        }
                        GeneratedField::TurnCoverage => {
                            if turn_coverage__.is_some() {
                                return Err(serde::de::Error::duplicate_field("turnCoverage"));
                            }
                            turn_coverage__ = map_.next_value::<::std::option::Option<realtime_input_config::TurnCoverage>>()?.map(|x| x as i32);
                        }
                    }
                }
                Ok(RealtimeInputConfig {
                    automatic_activity_detection: automatic_activity_detection__,
                    activity_handling: activity_handling__,
                    turn_coverage: turn_coverage__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.RealtimeInputConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for realtime_input_config::ActivityHandling {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ACTIVITY_HANDLING_UNSPECIFIED",
            Self::StartOfActivityInterrupts => "START_OF_ACTIVITY_INTERRUPTS",
            Self::NoInterruption => "NO_INTERRUPTION",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for realtime_input_config::ActivityHandling {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ACTIVITY_HANDLING_UNSPECIFIED",
            "START_OF_ACTIVITY_INTERRUPTS",
            "NO_INTERRUPTION",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = realtime_input_config::ActivityHandling;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ACTIVITY_HANDLING_UNSPECIFIED" => Ok(realtime_input_config::ActivityHandling::Unspecified),
                    "START_OF_ACTIVITY_INTERRUPTS" => Ok(realtime_input_config::ActivityHandling::StartOfActivityInterrupts),
                    "NO_INTERRUPTION" => Ok(realtime_input_config::ActivityHandling::NoInterruption),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for realtime_input_config::AutomaticActivityDetection {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.disabled.is_some() {
            len += 1;
        }
        if self.start_of_speech_sensitivity.is_some() {
            len += 1;
        }
        if self.prefix_padding_ms.is_some() {
            len += 1;
        }
        if self.end_of_speech_sensitivity.is_some() {
            len += 1;
        }
        if self.silence_duration_ms.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.RealtimeInputConfig.AutomaticActivityDetection", len)?;
        if let Some(v) = self.disabled.as_ref() {
            struct_ser.serialize_field("disabled", v)?;
        }
        if let Some(v) = self.start_of_speech_sensitivity.as_ref() {
            let v = realtime_input_config::automatic_activity_detection::StartSensitivity::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("startOfSpeechSensitivity", &v)?;
        }
        if let Some(v) = self.prefix_padding_ms.as_ref() {
            struct_ser.serialize_field("prefixPaddingMs", v)?;
        }
        if let Some(v) = self.end_of_speech_sensitivity.as_ref() {
            let v = realtime_input_config::automatic_activity_detection::EndSensitivity::try_from(*v)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
            struct_ser.serialize_field("endOfSpeechSensitivity", &v)?;
        }
        if let Some(v) = self.silence_duration_ms.as_ref() {
            struct_ser.serialize_field("silenceDurationMs", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for realtime_input_config::AutomaticActivityDetection {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "disabled",
            "start_of_speech_sensitivity",
            "startOfSpeechSensitivity",
            "prefix_padding_ms",
            "prefixPaddingMs",
            "end_of_speech_sensitivity",
            "endOfSpeechSensitivity",
            "silence_duration_ms",
            "silenceDurationMs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Disabled,
            StartOfSpeechSensitivity,
            PrefixPaddingMs,
            EndOfSpeechSensitivity,
            SilenceDurationMs,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "disabled" => Ok(GeneratedField::Disabled),
                            "startOfSpeechSensitivity" | "start_of_speech_sensitivity" => Ok(GeneratedField::StartOfSpeechSensitivity),
                            "prefixPaddingMs" | "prefix_padding_ms" => Ok(GeneratedField::PrefixPaddingMs),
                            "endOfSpeechSensitivity" | "end_of_speech_sensitivity" => Ok(GeneratedField::EndOfSpeechSensitivity),
                            "silenceDurationMs" | "silence_duration_ms" => Ok(GeneratedField::SilenceDurationMs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = realtime_input_config::AutomaticActivityDetection;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.RealtimeInputConfig.AutomaticActivityDetection")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<realtime_input_config::AutomaticActivityDetection, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut disabled__ = None;
                let mut start_of_speech_sensitivity__ = None;
                let mut prefix_padding_ms__ = None;
                let mut end_of_speech_sensitivity__ = None;
                let mut silence_duration_ms__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Disabled => {
                            if disabled__.is_some() {
                                return Err(serde::de::Error::duplicate_field("disabled"));
                            }
                            disabled__ = map_.next_value()?;
                        }
                        GeneratedField::StartOfSpeechSensitivity => {
                            if start_of_speech_sensitivity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startOfSpeechSensitivity"));
                            }
                            start_of_speech_sensitivity__ = map_.next_value::<::std::option::Option<realtime_input_config::automatic_activity_detection::StartSensitivity>>()?.map(|x| x as i32);
                        }
                        GeneratedField::PrefixPaddingMs => {
                            if prefix_padding_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prefixPaddingMs"));
                            }
                            prefix_padding_ms__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::EndOfSpeechSensitivity => {
                            if end_of_speech_sensitivity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endOfSpeechSensitivity"));
                            }
                            end_of_speech_sensitivity__ = map_.next_value::<::std::option::Option<realtime_input_config::automatic_activity_detection::EndSensitivity>>()?.map(|x| x as i32);
                        }
                        GeneratedField::SilenceDurationMs => {
                            if silence_duration_ms__.is_some() {
                                return Err(serde::de::Error::duplicate_field("silenceDurationMs"));
                            }
                            silence_duration_ms__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(realtime_input_config::AutomaticActivityDetection {
                    disabled: disabled__,
                    start_of_speech_sensitivity: start_of_speech_sensitivity__,
                    prefix_padding_ms: prefix_padding_ms__,
                    end_of_speech_sensitivity: end_of_speech_sensitivity__,
                    silence_duration_ms: silence_duration_ms__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.RealtimeInputConfig.AutomaticActivityDetection", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for realtime_input_config::automatic_activity_detection::EndSensitivity {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "END_SENSITIVITY_UNSPECIFIED",
            Self::High => "END_SENSITIVITY_HIGH",
            Self::Low => "END_SENSITIVITY_LOW",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for realtime_input_config::automatic_activity_detection::EndSensitivity {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "END_SENSITIVITY_UNSPECIFIED",
            "END_SENSITIVITY_HIGH",
            "END_SENSITIVITY_LOW",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = realtime_input_config::automatic_activity_detection::EndSensitivity;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "END_SENSITIVITY_UNSPECIFIED" => Ok(realtime_input_config::automatic_activity_detection::EndSensitivity::Unspecified),
                    "END_SENSITIVITY_HIGH" => Ok(realtime_input_config::automatic_activity_detection::EndSensitivity::High),
                    "END_SENSITIVITY_LOW" => Ok(realtime_input_config::automatic_activity_detection::EndSensitivity::Low),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for realtime_input_config::automatic_activity_detection::StartSensitivity {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "START_SENSITIVITY_UNSPECIFIED",
            Self::High => "START_SENSITIVITY_HIGH",
            Self::Low => "START_SENSITIVITY_LOW",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for realtime_input_config::automatic_activity_detection::StartSensitivity {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "START_SENSITIVITY_UNSPECIFIED",
            "START_SENSITIVITY_HIGH",
            "START_SENSITIVITY_LOW",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = realtime_input_config::automatic_activity_detection::StartSensitivity;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "START_SENSITIVITY_UNSPECIFIED" => Ok(realtime_input_config::automatic_activity_detection::StartSensitivity::Unspecified),
                    "START_SENSITIVITY_HIGH" => Ok(realtime_input_config::automatic_activity_detection::StartSensitivity::High),
                    "START_SENSITIVITY_LOW" => Ok(realtime_input_config::automatic_activity_detection::StartSensitivity::Low),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for realtime_input_config::TurnCoverage {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TURN_COVERAGE_UNSPECIFIED",
            Self::TurnIncludesOnlyActivity => "TURN_INCLUDES_ONLY_ACTIVITY",
            Self::TurnIncludesAllInput => "TURN_INCLUDES_ALL_INPUT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for realtime_input_config::TurnCoverage {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TURN_COVERAGE_UNSPECIFIED",
            "TURN_INCLUDES_ONLY_ACTIVITY",
            "TURN_INCLUDES_ALL_INPUT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = realtime_input_config::TurnCoverage;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TURN_COVERAGE_UNSPECIFIED" => Ok(realtime_input_config::TurnCoverage::Unspecified),
                    "TURN_INCLUDES_ONLY_ACTIVITY" => Ok(realtime_input_config::TurnCoverage::TurnIncludesOnlyActivity),
                    "TURN_INCLUDES_ALL_INPUT" => Ok(realtime_input_config::TurnCoverage::TurnIncludesAllInput),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for RetrievalConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.lat_lng.is_some() {
            len += 1;
        }
        if !self.language_code.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.RetrievalConfig", len)?;
        if let Some(v) = self.lat_lng.as_ref() {
            struct_ser.serialize_field("latLng", v)?;
        }
        if !self.language_code.is_empty() {
            struct_ser.serialize_field("languageCode", &self.language_code)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RetrievalConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "lat_lng",
            "latLng",
            "language_code",
            "languageCode",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            LatLng,
            LanguageCode,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "latLng" | "lat_lng" => Ok(GeneratedField::LatLng),
                            "languageCode" | "language_code" => Ok(GeneratedField::LanguageCode),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RetrievalConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.RetrievalConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RetrievalConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut lat_lng__ = None;
                let mut language_code__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::LatLng => {
                            if lat_lng__.is_some() {
                                return Err(serde::de::Error::duplicate_field("latLng"));
                            }
                            lat_lng__ = map_.next_value()?;
                        }
                        GeneratedField::LanguageCode => {
                            if language_code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("languageCode"));
                            }
                            language_code__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RetrievalConfig {
                    lat_lng: lat_lng__,
                    language_code: language_code__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.RetrievalConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RetrievalMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.google_search_dynamic_retrieval_score != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.RetrievalMetadata", len)?;
        if self.google_search_dynamic_retrieval_score != 0. {
            struct_ser.serialize_field("googleSearchDynamicRetrievalScore", &self.google_search_dynamic_retrieval_score)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RetrievalMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "google_search_dynamic_retrieval_score",
            "googleSearchDynamicRetrievalScore",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            GoogleSearchDynamicRetrievalScore,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "googleSearchDynamicRetrievalScore" | "google_search_dynamic_retrieval_score" => Ok(GeneratedField::GoogleSearchDynamicRetrievalScore),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RetrievalMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.RetrievalMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RetrievalMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut google_search_dynamic_retrieval_score__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::GoogleSearchDynamicRetrievalScore => {
                            if google_search_dynamic_retrieval_score__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleSearchDynamicRetrievalScore"));
                            }
                            google_search_dynamic_retrieval_score__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(RetrievalMetadata {
                    google_search_dynamic_retrieval_score: google_search_dynamic_retrieval_score__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.RetrievalMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SafetyFeedback {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.rating.is_some() {
            len += 1;
        }
        if self.setting.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SafetyFeedback", len)?;
        if let Some(v) = self.rating.as_ref() {
            struct_ser.serialize_field("rating", v)?;
        }
        if let Some(v) = self.setting.as_ref() {
            struct_ser.serialize_field("setting", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SafetyFeedback {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rating",
            "setting",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Rating,
            Setting,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "rating" => Ok(GeneratedField::Rating),
                            "setting" => Ok(GeneratedField::Setting),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SafetyFeedback;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SafetyFeedback")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SafetyFeedback, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rating__ = None;
                let mut setting__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Rating => {
                            if rating__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rating"));
                            }
                            rating__ = map_.next_value()?;
                        }
                        GeneratedField::Setting => {
                            if setting__.is_some() {
                                return Err(serde::de::Error::duplicate_field("setting"));
                            }
                            setting__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SafetyFeedback {
                    rating: rating__,
                    setting: setting__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SafetyFeedback", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SafetyRating {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.category != 0 {
            len += 1;
        }
        if self.probability != 0 {
            len += 1;
        }
        if self.blocked {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SafetyRating", len)?;
        if self.category != 0 {
            let v = HarmCategory::try_from(self.category)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.category)))?;
            struct_ser.serialize_field("category", &v)?;
        }
        if self.probability != 0 {
            let v = safety_rating::HarmProbability::try_from(self.probability)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.probability)))?;
            struct_ser.serialize_field("probability", &v)?;
        }
        if self.blocked {
            struct_ser.serialize_field("blocked", &self.blocked)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SafetyRating {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "category",
            "probability",
            "blocked",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Category,
            Probability,
            Blocked,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "category" => Ok(GeneratedField::Category),
                            "probability" => Ok(GeneratedField::Probability),
                            "blocked" => Ok(GeneratedField::Blocked),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SafetyRating;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SafetyRating")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SafetyRating, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut category__ = None;
                let mut probability__ = None;
                let mut blocked__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Category => {
                            if category__.is_some() {
                                return Err(serde::de::Error::duplicate_field("category"));
                            }
                            category__ = Some(map_.next_value::<HarmCategory>()? as i32);
                        }
                        GeneratedField::Probability => {
                            if probability__.is_some() {
                                return Err(serde::de::Error::duplicate_field("probability"));
                            }
                            probability__ = Some(map_.next_value::<safety_rating::HarmProbability>()? as i32);
                        }
                        GeneratedField::Blocked => {
                            if blocked__.is_some() {
                                return Err(serde::de::Error::duplicate_field("blocked"));
                            }
                            blocked__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SafetyRating {
                    category: category__.unwrap_or_default(),
                    probability: probability__.unwrap_or_default(),
                    blocked: blocked__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SafetyRating", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for safety_rating::HarmProbability {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "HARM_PROBABILITY_UNSPECIFIED",
            Self::Negligible => "NEGLIGIBLE",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for safety_rating::HarmProbability {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "HARM_PROBABILITY_UNSPECIFIED",
            "NEGLIGIBLE",
            "LOW",
            "MEDIUM",
            "HIGH",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = safety_rating::HarmProbability;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "HARM_PROBABILITY_UNSPECIFIED" => Ok(safety_rating::HarmProbability::Unspecified),
                    "NEGLIGIBLE" => Ok(safety_rating::HarmProbability::Negligible),
                    "LOW" => Ok(safety_rating::HarmProbability::Low),
                    "MEDIUM" => Ok(safety_rating::HarmProbability::Medium),
                    "HIGH" => Ok(safety_rating::HarmProbability::High),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for SafetySetting {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.category != 0 {
            len += 1;
        }
        if self.threshold != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SafetySetting", len)?;
        if self.category != 0 {
            let v = HarmCategory::try_from(self.category)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.category)))?;
            struct_ser.serialize_field("category", &v)?;
        }
        if self.threshold != 0 {
            let v = safety_setting::HarmBlockThreshold::try_from(self.threshold)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.threshold)))?;
            struct_ser.serialize_field("threshold", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SafetySetting {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "category",
            "threshold",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Category,
            Threshold,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "category" => Ok(GeneratedField::Category),
                            "threshold" => Ok(GeneratedField::Threshold),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SafetySetting;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SafetySetting")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SafetySetting, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut category__ = None;
                let mut threshold__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Category => {
                            if category__.is_some() {
                                return Err(serde::de::Error::duplicate_field("category"));
                            }
                            category__ = Some(map_.next_value::<HarmCategory>()? as i32);
                        }
                        GeneratedField::Threshold => {
                            if threshold__.is_some() {
                                return Err(serde::de::Error::duplicate_field("threshold"));
                            }
                            threshold__ = Some(map_.next_value::<safety_setting::HarmBlockThreshold>()? as i32);
                        }
                    }
                }
                Ok(SafetySetting {
                    category: category__.unwrap_or_default(),
                    threshold: threshold__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SafetySetting", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for safety_setting::HarmBlockThreshold {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "HARM_BLOCK_THRESHOLD_UNSPECIFIED",
            Self::BlockLowAndAbove => "BLOCK_LOW_AND_ABOVE",
            Self::BlockMediumAndAbove => "BLOCK_MEDIUM_AND_ABOVE",
            Self::BlockOnlyHigh => "BLOCK_ONLY_HIGH",
            Self::BlockNone => "BLOCK_NONE",
            Self::Off => "OFF",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for safety_setting::HarmBlockThreshold {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "HARM_BLOCK_THRESHOLD_UNSPECIFIED",
            "BLOCK_LOW_AND_ABOVE",
            "BLOCK_MEDIUM_AND_ABOVE",
            "BLOCK_ONLY_HIGH",
            "BLOCK_NONE",
            "OFF",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = safety_setting::HarmBlockThreshold;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "HARM_BLOCK_THRESHOLD_UNSPECIFIED" => Ok(safety_setting::HarmBlockThreshold::Unspecified),
                    "BLOCK_LOW_AND_ABOVE" => Ok(safety_setting::HarmBlockThreshold::BlockLowAndAbove),
                    "BLOCK_MEDIUM_AND_ABOVE" => Ok(safety_setting::HarmBlockThreshold::BlockMediumAndAbove),
                    "BLOCK_ONLY_HIGH" => Ok(safety_setting::HarmBlockThreshold::BlockOnlyHigh),
                    "BLOCK_NONE" => Ok(safety_setting::HarmBlockThreshold::BlockNone),
                    "OFF" => Ok(safety_setting::HarmBlockThreshold::Off),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Schema {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if !self.format.is_empty() {
            len += 1;
        }
        if !self.title.is_empty() {
            len += 1;
        }
        if !self.description.is_empty() {
            len += 1;
        }
        if self.nullable {
            len += 1;
        }
        if !self.r#enum.is_empty() {
            len += 1;
        }
        if self.items.is_some() {
            len += 1;
        }
        if self.max_items != 0 {
            len += 1;
        }
        if self.min_items != 0 {
            len += 1;
        }
        if !self.properties.is_empty() {
            len += 1;
        }
        if !self.required.is_empty() {
            len += 1;
        }
        if self.min_properties != 0 {
            len += 1;
        }
        if self.max_properties != 0 {
            len += 1;
        }
        if self.minimum.is_some() {
            len += 1;
        }
        if self.maximum.is_some() {
            len += 1;
        }
        if self.min_length != 0 {
            len += 1;
        }
        if self.max_length != 0 {
            len += 1;
        }
        if !self.pattern.is_empty() {
            len += 1;
        }
        if self.example.is_some() {
            len += 1;
        }
        if !self.any_of.is_empty() {
            len += 1;
        }
        if !self.property_ordering.is_empty() {
            len += 1;
        }
        if self.default.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Schema", len)?;
        if self.r#type != 0 {
            let v = Type::try_from(self.r#type)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if !self.format.is_empty() {
            struct_ser.serialize_field("format", &self.format)?;
        }
        if !self.title.is_empty() {
            struct_ser.serialize_field("title", &self.title)?;
        }
        if !self.description.is_empty() {
            struct_ser.serialize_field("description", &self.description)?;
        }
        if self.nullable {
            struct_ser.serialize_field("nullable", &self.nullable)?;
        }
        if !self.r#enum.is_empty() {
            struct_ser.serialize_field("enum", &self.r#enum)?;
        }
        if let Some(v) = self.items.as_ref() {
            struct_ser.serialize_field("items", v)?;
        }
        if self.max_items != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("maxItems", ToString::to_string(&self.max_items).as_str())?;
        }
        if self.min_items != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("minItems", ToString::to_string(&self.min_items).as_str())?;
        }
        if !self.properties.is_empty() {
            struct_ser.serialize_field("properties", &self.properties)?;
        }
        if !self.required.is_empty() {
            struct_ser.serialize_field("required", &self.required)?;
        }
        if self.min_properties != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("minProperties", ToString::to_string(&self.min_properties).as_str())?;
        }
        if self.max_properties != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("maxProperties", ToString::to_string(&self.max_properties).as_str())?;
        }
        if let Some(v) = self.minimum.as_ref() {
            struct_ser.serialize_field("minimum", v)?;
        }
        if let Some(v) = self.maximum.as_ref() {
            struct_ser.serialize_field("maximum", v)?;
        }
        if self.min_length != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("minLength", ToString::to_string(&self.min_length).as_str())?;
        }
        if self.max_length != 0 {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("maxLength", ToString::to_string(&self.max_length).as_str())?;
        }
        if !self.pattern.is_empty() {
            struct_ser.serialize_field("pattern", &self.pattern)?;
        }
        if let Some(v) = self.example.as_ref() {
            struct_ser.serialize_field("example", v)?;
        }
        if !self.any_of.is_empty() {
            struct_ser.serialize_field("anyOf", &self.any_of)?;
        }
        if !self.property_ordering.is_empty() {
            struct_ser.serialize_field("propertyOrdering", &self.property_ordering)?;
        }
        if let Some(v) = self.default.as_ref() {
            struct_ser.serialize_field("default", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Schema {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "format",
            "title",
            "description",
            "nullable",
            "enum",
            "items",
            "max_items",
            "maxItems",
            "min_items",
            "minItems",
            "properties",
            "required",
            "min_properties",
            "minProperties",
            "max_properties",
            "maxProperties",
            "minimum",
            "maximum",
            "min_length",
            "minLength",
            "max_length",
            "maxLength",
            "pattern",
            "example",
            "any_of",
            "anyOf",
            "property_ordering",
            "propertyOrdering",
            "default",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Format,
            Title,
            Description,
            Nullable,
            Enum,
            Items,
            MaxItems,
            MinItems,
            Properties,
            Required,
            MinProperties,
            MaxProperties,
            Minimum,
            Maximum,
            MinLength,
            MaxLength,
            Pattern,
            Example,
            AnyOf,
            PropertyOrdering,
            Default,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "format" => Ok(GeneratedField::Format),
                            "title" => Ok(GeneratedField::Title),
                            "description" => Ok(GeneratedField::Description),
                            "nullable" => Ok(GeneratedField::Nullable),
                            "enum" => Ok(GeneratedField::Enum),
                            "items" => Ok(GeneratedField::Items),
                            "maxItems" | "max_items" => Ok(GeneratedField::MaxItems),
                            "minItems" | "min_items" => Ok(GeneratedField::MinItems),
                            "properties" => Ok(GeneratedField::Properties),
                            "required" => Ok(GeneratedField::Required),
                            "minProperties" | "min_properties" => Ok(GeneratedField::MinProperties),
                            "maxProperties" | "max_properties" => Ok(GeneratedField::MaxProperties),
                            "minimum" => Ok(GeneratedField::Minimum),
                            "maximum" => Ok(GeneratedField::Maximum),
                            "minLength" | "min_length" => Ok(GeneratedField::MinLength),
                            "maxLength" | "max_length" => Ok(GeneratedField::MaxLength),
                            "pattern" => Ok(GeneratedField::Pattern),
                            "example" => Ok(GeneratedField::Example),
                            "anyOf" | "any_of" => Ok(GeneratedField::AnyOf),
                            "propertyOrdering" | "property_ordering" => Ok(GeneratedField::PropertyOrdering),
                            "default" => Ok(GeneratedField::Default),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Schema;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Schema")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Schema, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut format__ = None;
                let mut title__ = None;
                let mut description__ = None;
                let mut nullable__ = None;
                let mut r#enum__ = None;
                let mut items__ = None;
                let mut max_items__ = None;
                let mut min_items__ = None;
                let mut properties__ = None;
                let mut required__ = None;
                let mut min_properties__ = None;
                let mut max_properties__ = None;
                let mut minimum__ = None;
                let mut maximum__ = None;
                let mut min_length__ = None;
                let mut max_length__ = None;
                let mut pattern__ = None;
                let mut example__ = None;
                let mut any_of__ = None;
                let mut property_ordering__ = None;
                let mut default__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map_.next_value::<Type>()? as i32);
                        }
                        GeneratedField::Format => {
                            if format__.is_some() {
                                return Err(serde::de::Error::duplicate_field("format"));
                            }
                            format__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Description => {
                            if description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("description"));
                            }
                            description__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Nullable => {
                            if nullable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("nullable"));
                            }
                            nullable__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Enum => {
                            if r#enum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("enum"));
                            }
                            r#enum__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Items => {
                            if items__.is_some() {
                                return Err(serde::de::Error::duplicate_field("items"));
                            }
                            items__ = map_.next_value()?;
                        }
                        GeneratedField::MaxItems => {
                            if max_items__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxItems"));
                            }
                            max_items__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MinItems => {
                            if min_items__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minItems"));
                            }
                            min_items__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Properties => {
                            if properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("properties"));
                            }
                            properties__ = Some(
                                map_.next_value::<std::collections::HashMap<_, _>>()?
                            );
                        }
                        GeneratedField::Required => {
                            if required__.is_some() {
                                return Err(serde::de::Error::duplicate_field("required"));
                            }
                            required__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MinProperties => {
                            if min_properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minProperties"));
                            }
                            min_properties__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MaxProperties => {
                            if max_properties__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxProperties"));
                            }
                            max_properties__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Minimum => {
                            if minimum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minimum"));
                            }
                            minimum__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::Maximum => {
                            if maximum__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maximum"));
                            }
                            maximum__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::MinLength => {
                            if min_length__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minLength"));
                            }
                            min_length__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MaxLength => {
                            if max_length__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxLength"));
                            }
                            max_length__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Pattern => {
                            if pattern__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pattern"));
                            }
                            pattern__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Example => {
                            if example__.is_some() {
                                return Err(serde::de::Error::duplicate_field("example"));
                            }
                            example__ = map_.next_value()?;
                        }
                        GeneratedField::AnyOf => {
                            if any_of__.is_some() {
                                return Err(serde::de::Error::duplicate_field("anyOf"));
                            }
                            any_of__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PropertyOrdering => {
                            if property_ordering__.is_some() {
                                return Err(serde::de::Error::duplicate_field("propertyOrdering"));
                            }
                            property_ordering__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Default => {
                            if default__.is_some() {
                                return Err(serde::de::Error::duplicate_field("default"));
                            }
                            default__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Schema {
                    r#type: r#type__.unwrap_or_default(),
                    format: format__.unwrap_or_default(),
                    title: title__.unwrap_or_default(),
                    description: description__.unwrap_or_default(),
                    nullable: nullable__.unwrap_or_default(),
                    r#enum: r#enum__.unwrap_or_default(),
                    items: items__,
                    max_items: max_items__.unwrap_or_default(),
                    min_items: min_items__.unwrap_or_default(),
                    properties: properties__.unwrap_or_default(),
                    required: required__.unwrap_or_default(),
                    min_properties: min_properties__.unwrap_or_default(),
                    max_properties: max_properties__.unwrap_or_default(),
                    minimum: minimum__,
                    maximum: maximum__,
                    min_length: min_length__.unwrap_or_default(),
                    max_length: max_length__.unwrap_or_default(),
                    pattern: pattern__.unwrap_or_default(),
                    example: example__,
                    any_of: any_of__.unwrap_or_default(),
                    property_ordering: property_ordering__.unwrap_or_default(),
                    default: default__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Schema", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SearchEntryPoint {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.rendered_content.is_empty() {
            len += 1;
        }
        if !self.sdk_blob.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SearchEntryPoint", len)?;
        if !self.rendered_content.is_empty() {
            struct_ser.serialize_field("renderedContent", &self.rendered_content)?;
        }
        if !self.sdk_blob.is_empty() {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("sdkBlob", pbjson::private::base64::encode(&self.sdk_blob).as_str())?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SearchEntryPoint {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "rendered_content",
            "renderedContent",
            "sdk_blob",
            "sdkBlob",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RenderedContent,
            SdkBlob,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "renderedContent" | "rendered_content" => Ok(GeneratedField::RenderedContent),
                            "sdkBlob" | "sdk_blob" => Ok(GeneratedField::SdkBlob),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SearchEntryPoint;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SearchEntryPoint")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SearchEntryPoint, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut rendered_content__ = None;
                let mut sdk_blob__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RenderedContent => {
                            if rendered_content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("renderedContent"));
                            }
                            rendered_content__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SdkBlob => {
                            if sdk_blob__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sdkBlob"));
                            }
                            sdk_blob__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(SearchEntryPoint {
                    rendered_content: rendered_content__.unwrap_or_default(),
                    sdk_blob: sdk_blob__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SearchEntryPoint", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Segment {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.part_index != 0 {
            len += 1;
        }
        if self.start_index != 0 {
            len += 1;
        }
        if self.end_index != 0 {
            len += 1;
        }
        if !self.text.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Segment", len)?;
        if self.part_index != 0 {
            struct_ser.serialize_field("partIndex", &self.part_index)?;
        }
        if self.start_index != 0 {
            struct_ser.serialize_field("startIndex", &self.start_index)?;
        }
        if self.end_index != 0 {
            struct_ser.serialize_field("endIndex", &self.end_index)?;
        }
        if !self.text.is_empty() {
            struct_ser.serialize_field("text", &self.text)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Segment {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "part_index",
            "partIndex",
            "start_index",
            "startIndex",
            "end_index",
            "endIndex",
            "text",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PartIndex,
            StartIndex,
            EndIndex,
            Text,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "partIndex" | "part_index" => Ok(GeneratedField::PartIndex),
                            "startIndex" | "start_index" => Ok(GeneratedField::StartIndex),
                            "endIndex" | "end_index" => Ok(GeneratedField::EndIndex),
                            "text" => Ok(GeneratedField::Text),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Segment;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Segment")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Segment, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut part_index__ = None;
                let mut start_index__ = None;
                let mut end_index__ = None;
                let mut text__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PartIndex => {
                            if part_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("partIndex"));
                            }
                            part_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::StartIndex => {
                            if start_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startIndex"));
                            }
                            start_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EndIndex => {
                            if end_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endIndex"));
                            }
                            end_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Segment {
                    part_index: part_index__.unwrap_or_default(),
                    start_index: start_index__.unwrap_or_default(),
                    end_index: end_index__.unwrap_or_default(),
                    text: text__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Segment", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SemanticRetrieverConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.source.is_empty() {
            len += 1;
        }
        if self.query.is_some() {
            len += 1;
        }
        if !self.metadata_filters.is_empty() {
            len += 1;
        }
        if self.max_chunks_count.is_some() {
            len += 1;
        }
        if self.minimum_relevance_score.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SemanticRetrieverConfig", len)?;
        if !self.source.is_empty() {
            struct_ser.serialize_field("source", &self.source)?;
        }
        if let Some(v) = self.query.as_ref() {
            struct_ser.serialize_field("query", v)?;
        }
        if !self.metadata_filters.is_empty() {
            struct_ser.serialize_field("metadataFilters", &self.metadata_filters)?;
        }
        if let Some(v) = self.max_chunks_count.as_ref() {
            struct_ser.serialize_field("maxChunksCount", v)?;
        }
        if let Some(v) = self.minimum_relevance_score.as_ref() {
            struct_ser.serialize_field("minimumRelevanceScore", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SemanticRetrieverConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "source",
            "query",
            "metadata_filters",
            "metadataFilters",
            "max_chunks_count",
            "maxChunksCount",
            "minimum_relevance_score",
            "minimumRelevanceScore",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Source,
            Query,
            MetadataFilters,
            MaxChunksCount,
            MinimumRelevanceScore,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "source" => Ok(GeneratedField::Source),
                            "query" => Ok(GeneratedField::Query),
                            "metadataFilters" | "metadata_filters" => Ok(GeneratedField::MetadataFilters),
                            "maxChunksCount" | "max_chunks_count" => Ok(GeneratedField::MaxChunksCount),
                            "minimumRelevanceScore" | "minimum_relevance_score" => Ok(GeneratedField::MinimumRelevanceScore),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SemanticRetrieverConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SemanticRetrieverConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SemanticRetrieverConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut source__ = None;
                let mut query__ = None;
                let mut metadata_filters__ = None;
                let mut max_chunks_count__ = None;
                let mut minimum_relevance_score__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Source => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("source"));
                            }
                            source__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Query => {
                            if query__.is_some() {
                                return Err(serde::de::Error::duplicate_field("query"));
                            }
                            query__ = map_.next_value()?;
                        }
                        GeneratedField::MetadataFilters => {
                            if metadata_filters__.is_some() {
                                return Err(serde::de::Error::duplicate_field("metadataFilters"));
                            }
                            metadata_filters__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MaxChunksCount => {
                            if max_chunks_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("maxChunksCount"));
                            }
                            max_chunks_count__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::MinimumRelevanceScore => {
                            if minimum_relevance_score__.is_some() {
                                return Err(serde::de::Error::duplicate_field("minimumRelevanceScore"));
                            }
                            minimum_relevance_score__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(SemanticRetrieverConfig {
                    source: source__.unwrap_or_default(),
                    query: query__,
                    metadata_filters: metadata_filters__.unwrap_or_default(),
                    max_chunks_count: max_chunks_count__,
                    minimum_relevance_score: minimum_relevance_score__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SemanticRetrieverConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SessionResumptionConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.handle.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SessionResumptionConfig", len)?;
        if let Some(v) = self.handle.as_ref() {
            struct_ser.serialize_field("handle", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SessionResumptionConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "handle",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Handle,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "handle" => Ok(GeneratedField::Handle),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SessionResumptionConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SessionResumptionConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SessionResumptionConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut handle__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Handle => {
                            if handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("handle"));
                            }
                            handle__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SessionResumptionConfig {
                    handle: handle__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SessionResumptionConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SessionResumptionUpdate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.new_handle.is_empty() {
            len += 1;
        }
        if self.resumable {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SessionResumptionUpdate", len)?;
        if !self.new_handle.is_empty() {
            struct_ser.serialize_field("newHandle", &self.new_handle)?;
        }
        if self.resumable {
            struct_ser.serialize_field("resumable", &self.resumable)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SessionResumptionUpdate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "new_handle",
            "newHandle",
            "resumable",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            NewHandle,
            Resumable,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "newHandle" | "new_handle" => Ok(GeneratedField::NewHandle),
                            "resumable" => Ok(GeneratedField::Resumable),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SessionResumptionUpdate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SessionResumptionUpdate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SessionResumptionUpdate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut new_handle__ = None;
                let mut resumable__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::NewHandle => {
                            if new_handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("newHandle"));
                            }
                            new_handle__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Resumable => {
                            if resumable__.is_some() {
                                return Err(serde::de::Error::duplicate_field("resumable"));
                            }
                            resumable__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SessionResumptionUpdate {
                    new_handle: new_handle__.unwrap_or_default(),
                    resumable: resumable__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SessionResumptionUpdate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SpeakerVoiceConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.speaker.is_empty() {
            len += 1;
        }
        if self.voice_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SpeakerVoiceConfig", len)?;
        if !self.speaker.is_empty() {
            struct_ser.serialize_field("speaker", &self.speaker)?;
        }
        if let Some(v) = self.voice_config.as_ref() {
            struct_ser.serialize_field("voiceConfig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SpeakerVoiceConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "speaker",
            "voice_config",
            "voiceConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Speaker,
            VoiceConfig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "speaker" => Ok(GeneratedField::Speaker),
                            "voiceConfig" | "voice_config" => Ok(GeneratedField::VoiceConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SpeakerVoiceConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SpeakerVoiceConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SpeakerVoiceConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut speaker__ = None;
                let mut voice_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Speaker => {
                            if speaker__.is_some() {
                                return Err(serde::de::Error::duplicate_field("speaker"));
                            }
                            speaker__ = Some(map_.next_value()?);
                        }
                        GeneratedField::VoiceConfig => {
                            if voice_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voiceConfig"));
                            }
                            voice_config__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SpeakerVoiceConfig {
                    speaker: speaker__.unwrap_or_default(),
                    voice_config: voice_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SpeakerVoiceConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SpeechConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.voice_config.is_some() {
            len += 1;
        }
        if self.multi_speaker_voice_config.is_some() {
            len += 1;
        }
        if !self.language_code.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.SpeechConfig", len)?;
        if let Some(v) = self.voice_config.as_ref() {
            struct_ser.serialize_field("voiceConfig", v)?;
        }
        if let Some(v) = self.multi_speaker_voice_config.as_ref() {
            struct_ser.serialize_field("multiSpeakerVoiceConfig", v)?;
        }
        if !self.language_code.is_empty() {
            struct_ser.serialize_field("languageCode", &self.language_code)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SpeechConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "voice_config",
            "voiceConfig",
            "multi_speaker_voice_config",
            "multiSpeakerVoiceConfig",
            "language_code",
            "languageCode",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            VoiceConfig,
            MultiSpeakerVoiceConfig,
            LanguageCode,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "voiceConfig" | "voice_config" => Ok(GeneratedField::VoiceConfig),
                            "multiSpeakerVoiceConfig" | "multi_speaker_voice_config" => Ok(GeneratedField::MultiSpeakerVoiceConfig),
                            "languageCode" | "language_code" => Ok(GeneratedField::LanguageCode),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SpeechConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.SpeechConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SpeechConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut voice_config__ = None;
                let mut multi_speaker_voice_config__ = None;
                let mut language_code__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::VoiceConfig => {
                            if voice_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("voiceConfig"));
                            }
                            voice_config__ = map_.next_value()?;
                        }
                        GeneratedField::MultiSpeakerVoiceConfig => {
                            if multi_speaker_voice_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("multiSpeakerVoiceConfig"));
                            }
                            multi_speaker_voice_config__ = map_.next_value()?;
                        }
                        GeneratedField::LanguageCode => {
                            if language_code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("languageCode"));
                            }
                            language_code__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SpeechConfig {
                    voice_config: voice_config__,
                    multi_speaker_voice_config: multi_speaker_voice_config__,
                    language_code: language_code__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.SpeechConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StringList {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.values.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.StringList", len)?;
        if !self.values.is_empty() {
            struct_ser.serialize_field("values", &self.values)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StringList {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "values",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Values,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "values" => Ok(GeneratedField::Values),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StringList;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.StringList")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StringList, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut values__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Values => {
                            if values__.is_some() {
                                return Err(serde::de::Error::duplicate_field("values"));
                            }
                            values__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StringList {
                    values: values__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.StringList", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TaskType {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TASK_TYPE_UNSPECIFIED",
            Self::RetrievalQuery => "RETRIEVAL_QUERY",
            Self::RetrievalDocument => "RETRIEVAL_DOCUMENT",
            Self::SemanticSimilarity => "SEMANTIC_SIMILARITY",
            Self::Classification => "CLASSIFICATION",
            Self::Clustering => "CLUSTERING",
            Self::QuestionAnswering => "QUESTION_ANSWERING",
            Self::FactVerification => "FACT_VERIFICATION",
            Self::CodeRetrievalQuery => "CODE_RETRIEVAL_QUERY",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for TaskType {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TASK_TYPE_UNSPECIFIED",
            "RETRIEVAL_QUERY",
            "RETRIEVAL_DOCUMENT",
            "SEMANTIC_SIMILARITY",
            "CLASSIFICATION",
            "CLUSTERING",
            "QUESTION_ANSWERING",
            "FACT_VERIFICATION",
            "CODE_RETRIEVAL_QUERY",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TaskType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TASK_TYPE_UNSPECIFIED" => Ok(TaskType::Unspecified),
                    "RETRIEVAL_QUERY" => Ok(TaskType::RetrievalQuery),
                    "RETRIEVAL_DOCUMENT" => Ok(TaskType::RetrievalDocument),
                    "SEMANTIC_SIMILARITY" => Ok(TaskType::SemanticSimilarity),
                    "CLASSIFICATION" => Ok(TaskType::Classification),
                    "CLUSTERING" => Ok(TaskType::Clustering),
                    "QUESTION_ANSWERING" => Ok(TaskType::QuestionAnswering),
                    "FACT_VERIFICATION" => Ok(TaskType::FactVerification),
                    "CODE_RETRIEVAL_QUERY" => Ok(TaskType::CodeRetrievalQuery),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for ThinkingConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.include_thoughts.is_some() {
            len += 1;
        }
        if self.thinking_budget.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ThinkingConfig", len)?;
        if let Some(v) = self.include_thoughts.as_ref() {
            struct_ser.serialize_field("includeThoughts", v)?;
        }
        if let Some(v) = self.thinking_budget.as_ref() {
            struct_ser.serialize_field("thinkingBudget", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ThinkingConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "include_thoughts",
            "includeThoughts",
            "thinking_budget",
            "thinkingBudget",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            IncludeThoughts,
            ThinkingBudget,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "includeThoughts" | "include_thoughts" => Ok(GeneratedField::IncludeThoughts),
                            "thinkingBudget" | "thinking_budget" => Ok(GeneratedField::ThinkingBudget),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ThinkingConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ThinkingConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ThinkingConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut include_thoughts__ = None;
                let mut thinking_budget__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::IncludeThoughts => {
                            if include_thoughts__.is_some() {
                                return Err(serde::de::Error::duplicate_field("includeThoughts"));
                            }
                            include_thoughts__ = map_.next_value()?;
                        }
                        GeneratedField::ThinkingBudget => {
                            if thinking_budget__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thinkingBudget"));
                            }
                            thinking_budget__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(ThinkingConfig {
                    include_thoughts: include_thoughts__,
                    thinking_budget: thinking_budget__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ThinkingConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Tool {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.function_declarations.is_empty() {
            len += 1;
        }
        if self.google_search_retrieval.is_some() {
            len += 1;
        }
        if self.code_execution.is_some() {
            len += 1;
        }
        if self.google_search.is_some() {
            len += 1;
        }
        if self.computer_use.is_some() {
            len += 1;
        }
        if self.url_context.is_some() {
            len += 1;
        }
        if self.file_search.is_some() {
            len += 1;
        }
        if self.google_maps.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Tool", len)?;
        if !self.function_declarations.is_empty() {
            struct_ser.serialize_field("functionDeclarations", &self.function_declarations)?;
        }
        if let Some(v) = self.google_search_retrieval.as_ref() {
            struct_ser.serialize_field("googleSearchRetrieval", v)?;
        }
        if let Some(v) = self.code_execution.as_ref() {
            struct_ser.serialize_field("codeExecution", v)?;
        }
        if let Some(v) = self.google_search.as_ref() {
            struct_ser.serialize_field("googleSearch", v)?;
        }
        if let Some(v) = self.computer_use.as_ref() {
            struct_ser.serialize_field("computerUse", v)?;
        }
        if let Some(v) = self.url_context.as_ref() {
            struct_ser.serialize_field("urlContext", v)?;
        }
        if let Some(v) = self.file_search.as_ref() {
            struct_ser.serialize_field("fileSearch", v)?;
        }
        if let Some(v) = self.google_maps.as_ref() {
            struct_ser.serialize_field("googleMaps", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Tool {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function_declarations",
            "functionDeclarations",
            "google_search_retrieval",
            "googleSearchRetrieval",
            "code_execution",
            "codeExecution",
            "google_search",
            "googleSearch",
            "computer_use",
            "computerUse",
            "url_context",
            "urlContext",
            "file_search",
            "fileSearch",
            "google_maps",
            "googleMaps",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FunctionDeclarations,
            GoogleSearchRetrieval,
            CodeExecution,
            GoogleSearch,
            ComputerUse,
            UrlContext,
            FileSearch,
            GoogleMaps,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "functionDeclarations" | "function_declarations" => Ok(GeneratedField::FunctionDeclarations),
                            "googleSearchRetrieval" | "google_search_retrieval" => Ok(GeneratedField::GoogleSearchRetrieval),
                            "codeExecution" | "code_execution" => Ok(GeneratedField::CodeExecution),
                            "googleSearch" | "google_search" => Ok(GeneratedField::GoogleSearch),
                            "computerUse" | "computer_use" => Ok(GeneratedField::ComputerUse),
                            "urlContext" | "url_context" => Ok(GeneratedField::UrlContext),
                            "fileSearch" | "file_search" => Ok(GeneratedField::FileSearch),
                            "googleMaps" | "google_maps" => Ok(GeneratedField::GoogleMaps),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Tool;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Tool")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Tool, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function_declarations__ = None;
                let mut google_search_retrieval__ = None;
                let mut code_execution__ = None;
                let mut google_search__ = None;
                let mut computer_use__ = None;
                let mut url_context__ = None;
                let mut file_search__ = None;
                let mut google_maps__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::FunctionDeclarations => {
                            if function_declarations__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionDeclarations"));
                            }
                            function_declarations__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GoogleSearchRetrieval => {
                            if google_search_retrieval__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleSearchRetrieval"));
                            }
                            google_search_retrieval__ = map_.next_value()?;
                        }
                        GeneratedField::CodeExecution => {
                            if code_execution__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codeExecution"));
                            }
                            code_execution__ = map_.next_value()?;
                        }
                        GeneratedField::GoogleSearch => {
                            if google_search__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleSearch"));
                            }
                            google_search__ = map_.next_value()?;
                        }
                        GeneratedField::ComputerUse => {
                            if computer_use__.is_some() {
                                return Err(serde::de::Error::duplicate_field("computerUse"));
                            }
                            computer_use__ = map_.next_value()?;
                        }
                        GeneratedField::UrlContext => {
                            if url_context__.is_some() {
                                return Err(serde::de::Error::duplicate_field("urlContext"));
                            }
                            url_context__ = map_.next_value()?;
                        }
                        GeneratedField::FileSearch => {
                            if file_search__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fileSearch"));
                            }
                            file_search__ = map_.next_value()?;
                        }
                        GeneratedField::GoogleMaps => {
                            if google_maps__.is_some() {
                                return Err(serde::de::Error::duplicate_field("googleMaps"));
                            }
                            google_maps__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Tool {
                    function_declarations: function_declarations__.unwrap_or_default(),
                    google_search_retrieval: google_search_retrieval__,
                    code_execution: code_execution__,
                    google_search: google_search__,
                    computer_use: computer_use__,
                    url_context: url_context__,
                    file_search: file_search__,
                    google_maps: google_maps__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Tool", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for tool::ComputerUse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.environment != 0 {
            len += 1;
        }
        if !self.excluded_predefined_functions.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Tool.ComputerUse", len)?;
        if self.environment != 0 {
            let v = tool::computer_use::Environment::try_from(self.environment)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.environment)))?;
            struct_ser.serialize_field("environment", &v)?;
        }
        if !self.excluded_predefined_functions.is_empty() {
            struct_ser.serialize_field("excludedPredefinedFunctions", &self.excluded_predefined_functions)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for tool::ComputerUse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "environment",
            "excluded_predefined_functions",
            "excludedPredefinedFunctions",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Environment,
            ExcludedPredefinedFunctions,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "environment" => Ok(GeneratedField::Environment),
                            "excludedPredefinedFunctions" | "excluded_predefined_functions" => Ok(GeneratedField::ExcludedPredefinedFunctions),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = tool::ComputerUse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Tool.ComputerUse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<tool::ComputerUse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut environment__ = None;
                let mut excluded_predefined_functions__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Environment => {
                            if environment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("environment"));
                            }
                            environment__ = Some(map_.next_value::<tool::computer_use::Environment>()? as i32);
                        }
                        GeneratedField::ExcludedPredefinedFunctions => {
                            if excluded_predefined_functions__.is_some() {
                                return Err(serde::de::Error::duplicate_field("excludedPredefinedFunctions"));
                            }
                            excluded_predefined_functions__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(tool::ComputerUse {
                    environment: environment__.unwrap_or_default(),
                    excluded_predefined_functions: excluded_predefined_functions__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Tool.ComputerUse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for tool::computer_use::Environment {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "ENVIRONMENT_UNSPECIFIED",
            Self::Browser => "ENVIRONMENT_BROWSER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for tool::computer_use::Environment {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ENVIRONMENT_UNSPECIFIED",
            "ENVIRONMENT_BROWSER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = tool::computer_use::Environment;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "ENVIRONMENT_UNSPECIFIED" => Ok(tool::computer_use::Environment::Unspecified),
                    "ENVIRONMENT_BROWSER" => Ok(tool::computer_use::Environment::Browser),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for tool::GoogleSearch {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.time_range_filter.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.Tool.GoogleSearch", len)?;
        if let Some(v) = self.time_range_filter.as_ref() {
            struct_ser.serialize_field("timeRangeFilter", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for tool::GoogleSearch {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "time_range_filter",
            "timeRangeFilter",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TimeRangeFilter,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "timeRangeFilter" | "time_range_filter" => Ok(GeneratedField::TimeRangeFilter),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = tool::GoogleSearch;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.Tool.GoogleSearch")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<tool::GoogleSearch, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut time_range_filter__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TimeRangeFilter => {
                            if time_range_filter__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeRangeFilter"));
                            }
                            time_range_filter__ = map_.next_value()?;
                        }
                    }
                }
                Ok(tool::GoogleSearch {
                    time_range_filter: time_range_filter__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.Tool.GoogleSearch", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ToolConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.function_calling_config.is_some() {
            len += 1;
        }
        if self.retrieval_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.ToolConfig", len)?;
        if let Some(v) = self.function_calling_config.as_ref() {
            struct_ser.serialize_field("functionCallingConfig", v)?;
        }
        if let Some(v) = self.retrieval_config.as_ref() {
            struct_ser.serialize_field("retrievalConfig", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ToolConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "function_calling_config",
            "functionCallingConfig",
            "retrieval_config",
            "retrievalConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            FunctionCallingConfig,
            RetrievalConfig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "functionCallingConfig" | "function_calling_config" => Ok(GeneratedField::FunctionCallingConfig),
                            "retrievalConfig" | "retrieval_config" => Ok(GeneratedField::RetrievalConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ToolConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.ToolConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ToolConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut function_calling_config__ = None;
                let mut retrieval_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::FunctionCallingConfig => {
                            if function_calling_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("functionCallingConfig"));
                            }
                            function_calling_config__ = map_.next_value()?;
                        }
                        GeneratedField::RetrievalConfig => {
                            if retrieval_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievalConfig"));
                            }
                            retrieval_config__ = map_.next_value()?;
                        }
                    }
                }
                Ok(ToolConfig {
                    function_calling_config: function_calling_config__,
                    retrieval_config: retrieval_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.ToolConfig", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TYPE_UNSPECIFIED",
            Self::String => "STRING",
            Self::Number => "NUMBER",
            Self::Integer => "INTEGER",
            Self::Boolean => "BOOLEAN",
            Self::Array => "ARRAY",
            Self::Object => "OBJECT",
            Self::Null => "NULL",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TYPE_UNSPECIFIED",
            "STRING",
            "NUMBER",
            "INTEGER",
            "BOOLEAN",
            "ARRAY",
            "OBJECT",
            "NULL",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "TYPE_UNSPECIFIED" => Ok(Type::Unspecified),
                    "STRING" => Ok(Type::String),
                    "NUMBER" => Ok(Type::Number),
                    "INTEGER" => Ok(Type::Integer),
                    "BOOLEAN" => Ok(Type::Boolean),
                    "ARRAY" => Ok(Type::Array),
                    "OBJECT" => Ok(Type::Object),
                    "NULL" => Ok(Type::Null),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for UrlContext {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.UrlContext", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UrlContext {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UrlContext;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.UrlContext")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UrlContext, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(UrlContext {
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.UrlContext", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UrlContextMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.url_metadata.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.UrlContextMetadata", len)?;
        if !self.url_metadata.is_empty() {
            struct_ser.serialize_field("urlMetadata", &self.url_metadata)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UrlContextMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "url_metadata",
            "urlMetadata",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            UrlMetadata,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "urlMetadata" | "url_metadata" => Ok(GeneratedField::UrlMetadata),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UrlContextMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.UrlContextMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UrlContextMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut url_metadata__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::UrlMetadata => {
                            if url_metadata__.is_some() {
                                return Err(serde::de::Error::duplicate_field("urlMetadata"));
                            }
                            url_metadata__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(UrlContextMetadata {
                    url_metadata: url_metadata__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.UrlContextMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UrlMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.retrieved_url.is_empty() {
            len += 1;
        }
        if self.url_retrieval_status != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.UrlMetadata", len)?;
        if !self.retrieved_url.is_empty() {
            struct_ser.serialize_field("retrievedUrl", &self.retrieved_url)?;
        }
        if self.url_retrieval_status != 0 {
            let v = url_metadata::UrlRetrievalStatus::try_from(self.url_retrieval_status)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.url_retrieval_status)))?;
            struct_ser.serialize_field("urlRetrievalStatus", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UrlMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "retrieved_url",
            "retrievedUrl",
            "url_retrieval_status",
            "urlRetrievalStatus",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            RetrievedUrl,
            UrlRetrievalStatus,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "retrievedUrl" | "retrieved_url" => Ok(GeneratedField::RetrievedUrl),
                            "urlRetrievalStatus" | "url_retrieval_status" => Ok(GeneratedField::UrlRetrievalStatus),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UrlMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.UrlMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UrlMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut retrieved_url__ = None;
                let mut url_retrieval_status__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::RetrievedUrl => {
                            if retrieved_url__.is_some() {
                                return Err(serde::de::Error::duplicate_field("retrievedUrl"));
                            }
                            retrieved_url__ = Some(map_.next_value()?);
                        }
                        GeneratedField::UrlRetrievalStatus => {
                            if url_retrieval_status__.is_some() {
                                return Err(serde::de::Error::duplicate_field("urlRetrievalStatus"));
                            }
                            url_retrieval_status__ = Some(map_.next_value::<url_metadata::UrlRetrievalStatus>()? as i32);
                        }
                    }
                }
                Ok(UrlMetadata {
                    retrieved_url: retrieved_url__.unwrap_or_default(),
                    url_retrieval_status: url_retrieval_status__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.UrlMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for url_metadata::UrlRetrievalStatus {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "URL_RETRIEVAL_STATUS_UNSPECIFIED",
            Self::Success => "URL_RETRIEVAL_STATUS_SUCCESS",
            Self::Error => "URL_RETRIEVAL_STATUS_ERROR",
            Self::Paywall => "URL_RETRIEVAL_STATUS_PAYWALL",
            Self::Unsafe => "URL_RETRIEVAL_STATUS_UNSAFE",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for url_metadata::UrlRetrievalStatus {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "URL_RETRIEVAL_STATUS_UNSPECIFIED",
            "URL_RETRIEVAL_STATUS_SUCCESS",
            "URL_RETRIEVAL_STATUS_ERROR",
            "URL_RETRIEVAL_STATUS_PAYWALL",
            "URL_RETRIEVAL_STATUS_UNSAFE",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = url_metadata::UrlRetrievalStatus;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "URL_RETRIEVAL_STATUS_UNSPECIFIED" => Ok(url_metadata::UrlRetrievalStatus::Unspecified),
                    "URL_RETRIEVAL_STATUS_SUCCESS" => Ok(url_metadata::UrlRetrievalStatus::Success),
                    "URL_RETRIEVAL_STATUS_ERROR" => Ok(url_metadata::UrlRetrievalStatus::Error),
                    "URL_RETRIEVAL_STATUS_PAYWALL" => Ok(url_metadata::UrlRetrievalStatus::Paywall),
                    "URL_RETRIEVAL_STATUS_UNSAFE" => Ok(url_metadata::UrlRetrievalStatus::Unsafe),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for UsageMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.prompt_token_count != 0 {
            len += 1;
        }
        if self.cached_content_token_count != 0 {
            len += 1;
        }
        if self.response_token_count != 0 {
            len += 1;
        }
        if self.tool_use_prompt_token_count != 0 {
            len += 1;
        }
        if self.thoughts_token_count != 0 {
            len += 1;
        }
        if self.total_token_count != 0 {
            len += 1;
        }
        if !self.prompt_tokens_details.is_empty() {
            len += 1;
        }
        if !self.cache_tokens_details.is_empty() {
            len += 1;
        }
        if !self.response_tokens_details.is_empty() {
            len += 1;
        }
        if !self.tool_use_prompt_tokens_details.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.UsageMetadata", len)?;
        if self.prompt_token_count != 0 {
            struct_ser.serialize_field("promptTokenCount", &self.prompt_token_count)?;
        }
        if self.cached_content_token_count != 0 {
            struct_ser.serialize_field("cachedContentTokenCount", &self.cached_content_token_count)?;
        }
        if self.response_token_count != 0 {
            struct_ser.serialize_field("responseTokenCount", &self.response_token_count)?;
        }
        if self.tool_use_prompt_token_count != 0 {
            struct_ser.serialize_field("toolUsePromptTokenCount", &self.tool_use_prompt_token_count)?;
        }
        if self.thoughts_token_count != 0 {
            struct_ser.serialize_field("thoughtsTokenCount", &self.thoughts_token_count)?;
        }
        if self.total_token_count != 0 {
            struct_ser.serialize_field("totalTokenCount", &self.total_token_count)?;
        }
        if !self.prompt_tokens_details.is_empty() {
            struct_ser.serialize_field("promptTokensDetails", &self.prompt_tokens_details)?;
        }
        if !self.cache_tokens_details.is_empty() {
            struct_ser.serialize_field("cacheTokensDetails", &self.cache_tokens_details)?;
        }
        if !self.response_tokens_details.is_empty() {
            struct_ser.serialize_field("responseTokensDetails", &self.response_tokens_details)?;
        }
        if !self.tool_use_prompt_tokens_details.is_empty() {
            struct_ser.serialize_field("toolUsePromptTokensDetails", &self.tool_use_prompt_tokens_details)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UsageMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prompt_token_count",
            "promptTokenCount",
            "cached_content_token_count",
            "cachedContentTokenCount",
            "response_token_count",
            "responseTokenCount",
            "tool_use_prompt_token_count",
            "toolUsePromptTokenCount",
            "thoughts_token_count",
            "thoughtsTokenCount",
            "total_token_count",
            "totalTokenCount",
            "prompt_tokens_details",
            "promptTokensDetails",
            "cache_tokens_details",
            "cacheTokensDetails",
            "response_tokens_details",
            "responseTokensDetails",
            "tool_use_prompt_tokens_details",
            "toolUsePromptTokensDetails",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PromptTokenCount,
            CachedContentTokenCount,
            ResponseTokenCount,
            ToolUsePromptTokenCount,
            ThoughtsTokenCount,
            TotalTokenCount,
            PromptTokensDetails,
            CacheTokensDetails,
            ResponseTokensDetails,
            ToolUsePromptTokensDetails,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "promptTokenCount" | "prompt_token_count" => Ok(GeneratedField::PromptTokenCount),
                            "cachedContentTokenCount" | "cached_content_token_count" => Ok(GeneratedField::CachedContentTokenCount),
                            "responseTokenCount" | "response_token_count" => Ok(GeneratedField::ResponseTokenCount),
                            "toolUsePromptTokenCount" | "tool_use_prompt_token_count" => Ok(GeneratedField::ToolUsePromptTokenCount),
                            "thoughtsTokenCount" | "thoughts_token_count" => Ok(GeneratedField::ThoughtsTokenCount),
                            "totalTokenCount" | "total_token_count" => Ok(GeneratedField::TotalTokenCount),
                            "promptTokensDetails" | "prompt_tokens_details" => Ok(GeneratedField::PromptTokensDetails),
                            "cacheTokensDetails" | "cache_tokens_details" => Ok(GeneratedField::CacheTokensDetails),
                            "responseTokensDetails" | "response_tokens_details" => Ok(GeneratedField::ResponseTokensDetails),
                            "toolUsePromptTokensDetails" | "tool_use_prompt_tokens_details" => Ok(GeneratedField::ToolUsePromptTokensDetails),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UsageMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.UsageMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UsageMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prompt_token_count__ = None;
                let mut cached_content_token_count__ = None;
                let mut response_token_count__ = None;
                let mut tool_use_prompt_token_count__ = None;
                let mut thoughts_token_count__ = None;
                let mut total_token_count__ = None;
                let mut prompt_tokens_details__ = None;
                let mut cache_tokens_details__ = None;
                let mut response_tokens_details__ = None;
                let mut tool_use_prompt_tokens_details__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PromptTokenCount => {
                            if prompt_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptTokenCount"));
                            }
                            prompt_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::CachedContentTokenCount => {
                            if cached_content_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cachedContentTokenCount"));
                            }
                            cached_content_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ResponseTokenCount => {
                            if response_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseTokenCount"));
                            }
                            response_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ToolUsePromptTokenCount => {
                            if tool_use_prompt_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolUsePromptTokenCount"));
                            }
                            tool_use_prompt_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ThoughtsTokenCount => {
                            if thoughts_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("thoughtsTokenCount"));
                            }
                            thoughts_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TotalTokenCount => {
                            if total_token_count__.is_some() {
                                return Err(serde::de::Error::duplicate_field("totalTokenCount"));
                            }
                            total_token_count__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::PromptTokensDetails => {
                            if prompt_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptTokensDetails"));
                            }
                            prompt_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::CacheTokensDetails => {
                            if cache_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cacheTokensDetails"));
                            }
                            cache_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ResponseTokensDetails => {
                            if response_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("responseTokensDetails"));
                            }
                            response_tokens_details__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ToolUsePromptTokensDetails => {
                            if tool_use_prompt_tokens_details__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolUsePromptTokensDetails"));
                            }
                            tool_use_prompt_tokens_details__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(UsageMetadata {
                    prompt_token_count: prompt_token_count__.unwrap_or_default(),
                    cached_content_token_count: cached_content_token_count__.unwrap_or_default(),
                    response_token_count: response_token_count__.unwrap_or_default(),
                    tool_use_prompt_token_count: tool_use_prompt_token_count__.unwrap_or_default(),
                    thoughts_token_count: thoughts_token_count__.unwrap_or_default(),
                    total_token_count: total_token_count__.unwrap_or_default(),
                    prompt_tokens_details: prompt_tokens_details__.unwrap_or_default(),
                    cache_tokens_details: cache_tokens_details__.unwrap_or_default(),
                    response_tokens_details: response_tokens_details__.unwrap_or_default(),
                    tool_use_prompt_tokens_details: tool_use_prompt_tokens_details__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.UsageMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VideoMetadata {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_offset.is_some() {
            len += 1;
        }
        if self.end_offset.is_some() {
            len += 1;
        }
        if self.fps != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.VideoMetadata", len)?;
        if let Some(v) = self.start_offset.as_ref() {
            struct_ser.serialize_field("startOffset", v)?;
        }
        if let Some(v) = self.end_offset.as_ref() {
            struct_ser.serialize_field("endOffset", v)?;
        }
        if self.fps != 0. {
            struct_ser.serialize_field("fps", &self.fps)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VideoMetadata {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_offset",
            "startOffset",
            "end_offset",
            "endOffset",
            "fps",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartOffset,
            EndOffset,
            Fps,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "startOffset" | "start_offset" => Ok(GeneratedField::StartOffset),
                            "endOffset" | "end_offset" => Ok(GeneratedField::EndOffset),
                            "fps" => Ok(GeneratedField::Fps),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VideoMetadata;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.VideoMetadata")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VideoMetadata, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_offset__ = None;
                let mut end_offset__ = None;
                let mut fps__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartOffset => {
                            if start_offset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startOffset"));
                            }
                            start_offset__ = map_.next_value()?;
                        }
                        GeneratedField::EndOffset => {
                            if end_offset__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endOffset"));
                            }
                            end_offset__ = map_.next_value()?;
                        }
                        GeneratedField::Fps => {
                            if fps__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fps"));
                            }
                            fps__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(VideoMetadata {
                    start_offset: start_offset__,
                    end_offset: end_offset__,
                    fps: fps__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.VideoMetadata", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VoiceConfig {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.voice_config.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.ai.generativelanguage.v1beta.VoiceConfig", len)?;
        if let Some(v) = self.voice_config.as_ref() {
            match v {
                voice_config::VoiceConfig::PrebuiltVoiceConfig(v) => {
                    struct_ser.serialize_field("prebuiltVoiceConfig", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VoiceConfig {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prebuilt_voice_config",
            "prebuiltVoiceConfig",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PrebuiltVoiceConfig,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "prebuiltVoiceConfig" | "prebuilt_voice_config" => Ok(GeneratedField::PrebuiltVoiceConfig),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VoiceConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.ai.generativelanguage.v1beta.VoiceConfig")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VoiceConfig, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut voice_config__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PrebuiltVoiceConfig => {
                            if voice_config__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prebuiltVoiceConfig"));
                            }
                            voice_config__ = map_.next_value::<::std::option::Option<_>>()?.map(voice_config::VoiceConfig::PrebuiltVoiceConfig)
;
                        }
                    }
                }
                Ok(VoiceConfig {
                    voice_config: voice_config__,
                })
            }
        }
        deserializer.deserialize_struct("google.ai.generativelanguage.v1beta.VoiceConfig", FIELDS, GeneratedVisitor)
    }
}

impl serde::Serialize for Interval {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.start_time.is_some() {
            len += 1;
        }
        if self.end_time.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.r#type.Interval", len)?;
        if let Some(v) = self.start_time.as_ref() {
            struct_ser.serialize_field("startTime", v)?;
        }
        if let Some(v) = self.end_time.as_ref() {
            struct_ser.serialize_field("endTime", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Interval {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_time",
            "startTime",
            "end_time",
            "endTime",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartTime,
            EndTime,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "startTime" | "start_time" => Ok(GeneratedField::StartTime),
                            "endTime" | "end_time" => Ok(GeneratedField::EndTime),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Interval;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.r#type.Interval")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Interval, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_time__ = None;
                let mut end_time__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartTime => {
                            if start_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTime"));
                            }
                            start_time__ = map_.next_value()?;
                        }
                        GeneratedField::EndTime => {
                            if end_time__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endTime"));
                            }
                            end_time__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Interval {
                    start_time: start_time__,
                    end_time: end_time__,
                })
            }
        }
        deserializer.deserialize_struct("google.r#type.Interval", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for LatLng {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.latitude != 0. {
            len += 1;
        }
        if self.longitude != 0. {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("google.r#type.LatLng", len)?;
        if self.latitude != 0. {
            struct_ser.serialize_field("latitude", &self.latitude)?;
        }
        if self.longitude != 0. {
            struct_ser.serialize_field("longitude", &self.longitude)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for LatLng {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "latitude",
            "longitude",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Latitude,
            Longitude,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "latitude" => Ok(GeneratedField::Latitude),
                            "longitude" => Ok(GeneratedField::Longitude),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = LatLng;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct google.r#type.LatLng")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<LatLng, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut latitude__ = None;
                let mut longitude__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Latitude => {
                            if latitude__.is_some() {
                                return Err(serde::de::Error::duplicate_field("latitude"));
                            }
                            latitude__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Longitude => {
                            if longitude__.is_some() {
                                return Err(serde::de::Error::duplicate_field("longitude"));
                            }
                            longitude__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(LatLng {
                    latitude: latitude__.unwrap_or_default(),
                    longitude: longitude__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("google.r#type.LatLng", FIELDS, GeneratedVisitor)
    }
}

