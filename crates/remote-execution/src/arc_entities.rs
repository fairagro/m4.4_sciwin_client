use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ArcWorkflow {
    // Required fields
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "additionalType")]
    pub additional_type: String,
    pub identifier: String,
    #[serde(rename = "mainEntity")]
    pub main_entity: MainEntity,

    // Recommended fields
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "hasPart")]
    pub has_part: Option<Vec<String>>,

    // Optional fields
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MainEntity {
    #[serde(rename = "@id")]
    pub id: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowProtocol {
    // Required fields
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    //schema.org/Text AND schema.org/SoftwareSourceCode AND bioschemas.org/ComputationalWorkflow AND bioschemas.org/LabProtocol
    pub type_: Vec<String>,
    //MUST be 'Workflow Protocol' or ontology term to identify it as a Workflow Protocol
    #[serde(rename = "additionalType")]
    pub additional_type: String,

    // Recommended fields
    //From the workflow.cwl or run.cwl file.
    pub input: Option<Vec<Value>>,
    //From the workflow.cwl or run.cwl file.
    pub output: Option<Vec<Value>>,
    #[serde(rename = "dct:conformsTo")]
    pub dct_conforms_to: Option<String>,
    pub creator: Option<Vec<String>>,
    #[serde(rename = "dateCreated")]
    pub date_created: Option<String>,
    pub license: Option<Vec<String>>,
    pub name: Option<String>,
    #[serde(rename = "programmingLanguage")]
    pub programming_language: Option<Vec<String>>,
    #[serde(rename = "sdPublisher")]
    pub sd_publisher: Option<String>,
    pub url: Option<String>,
    pub version: Option<String>,

    // Optional fields
    pub description: Option<String>,
    #[serde(rename = "hasPart")]
    pub has_part: Option<Vec<String>>,
    #[serde(rename = "intendedUse")]
    pub intended_use: Option<String>,
    pub comment: Option<Vec<String>>,
    //Component entities from the Annotation Tables in the isa.workflow.xlsx
    //schema.org/SoftwareApplication OR schema.org/DefinedTerm OR schema.org/PropertyValue
    #[serde(rename = "computationalTool")]
    pub computational_tool: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArcRun {
    // Required fields
    //path to run folder
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "additionalType")]
    pub additional_type: String,
    pub identifier: String,

    // Recommended fields
    pub name: Option<String>,
    pub description: Option<String>,
    pub about: Option<Value>,
    pub mentions: Option<Value>,
    //People objects defined in the RUN PERFORMERS section of the isa.run.xlsx file
    pub creator: Option<Vec<String>>,
    //Colletion of all data entities defined in the Annotation Tables of the isa.run.xlsx file
    #[serde(rename = "hasPart")]
    pub has_part: Option<Vec<String>>,
    #[serde(rename = "measurementMethod")]
    pub measurement_method: Option<String>,
    #[serde(rename = "measurementTechnique")]
    pub measurement_technique: Option<String>,
    #[serde(rename = "conformsTo")]
    pub conforms_to: Option<Vec<String>>,

    // Optional fields
    //schema.org/URL
    pub url: Option<String>,
    //schema.org/Text OR schema.org/PropertyValue
    //A term to qualify the endpoint, or what is being computed (e.g. gene expression profiling or protein identification). 
    //The term can be free text or from, for example, a controlled vocabulary or an ontology.
    #[serde(rename = "variableMeasured")]
    pub variable_measured: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowInvocation {
    // Required fields
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    //schema.org/CreateAction AND bioschemas.org/LabProcess
    //MUST be LabProcess and CreateAction to indicate that this tool created the result data entities
    pub type_: Vec<String>,
    //MUST be 'Workflow Invocation' or ontology term to identify it as a Workflow Invocation
    #[serde(rename = "additionalType")]
    pub additional_type: String,
    pub name: String,
    // schema.org/MediaObject OR schema.org/Dataset OR schema.org/Collection OR schema.org/CreativeWork OR schema.org/PropertyValue
    // ids of entities that were consumed by action
    pub object: Vec<Value>,
    // schema.org/MediaObject OR schema.org/Dataset OR schema.org/Collection OR schema.org/CreativeWork OR schema.org/PropertyValue
    // id of entity created or modified by this action, e.g. output files
    pub result: Vec<Value>,
    //WorkflowProtocol
    // The executed Workflow Protocol. MUST follow the Workflow Protocol profile. MUST be equal to the executesLabProtocol property.
    pub instrument: Vec<Value>,
    //WorkflowProtocol
    // The executed Workflow Protocol. MUST follow the Workflow Protocol profile. MUST be equal to the instrument property.
    #[serde(rename = "executesLabProtocol")]
    pub executes_lab_protocol: Value,

    

    // Optional fields
    #[serde(rename = "parameterValue")]
    pub parameter_value: Option<Vec<Value>>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormalParameter {
    // Required fields
    // IRI
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    // bioschemas.org/FormalParameter
    pub type_: String,
    //SHOULD include: File, Dataset or Collection if it maps to a file, directory or multi-file dataset, respectively; 
    //PropertyValue if it maps to a dictionary-like structured value (e.g. a CWL record); DataType or one of its subtypes (e.g. Integer) if it maps to a non-structured value.
    #[serde(rename = "additionalType")]
    pub additional_type: String,

    // Recommended fields
    #[serde(rename = "dct:conformsTo")]
    pub dct_conforms_to: Option<String>,
    #[serde(rename = "encodingFormat")]
    pub encoding_format: Option<Vec<String>>,
    pub name: Option<String>,

    // Optional fields
    pub description: Option<String>,
    #[serde(rename = "workExample")]
    pub work_example: Option<String>,
    #[serde(rename = "defaultValue")]
    pub default_value: Option<String>,
    #[serde(rename = "valueRequired")]
    pub value_required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyValue {
    // Required fields
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: Vec<String>,
    #[serde(rename = "additionalType")]
    pub additional_type: String,
    #[serde(rename = "exampleOfWork")]
    pub example_of_work: String,
    pub value: String,

    // Recommended fields
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoftwareApplication {
    // Required fields
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: Vec<String>,

    // Recommended fields
    pub name: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "softwareVersion")]
    pub software_version: Option<String>,

    // Optional fields
    pub version: Option<String>,
    #[serde(rename = "applicationCategory")]
    pub application_category: Option<Vec<String>>,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<Vec<String>>,
    #[serde(rename = "softwareRequirements")]
    pub software_requirements: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArcRoCrate {
    #[serde(rename = "@context")]
    pub context: Vec<Value>,
    #[serde(rename = "@graph")]
    pub graph: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetEntity {
    #[serde(rename = "@id")]
    pub id: String,
    pub additional_type: Option<String>,
    pub identifier: Option<String>,
    pub date_modified: Option<String>,
    pub name: Option<String>,
    pub creator: Option<Value>,
    pub main_entity: Option<Value>,
    pub description: Option<String>,
    pub has_part: Option<Vec<Value>>,
    pub mentions: Option<Value>,
    pub about: Option<Value>,
    pub conforms_to: Option<Vec<Value>>,
    pub date_published: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DefinedTermEntity {
    #[serde(rename = "@id")]
    pub id: String,
    pub name: Option<String>,
    pub term_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "givenName")]
    pub given_name: Option<String>,
    #[serde(rename = "familyName")]
    pub family_name: Option<String>,
    #[serde(rename = "additionalName")]
    pub additional_name: Option<String>,
    pub affiliation: Option<Value>,
    pub email: Option<String>,
    #[serde(rename = "jobTitle")]
    pub job_title: Option<Value>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComputerLanguageEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    pub name: Option<String>,
    #[serde(rename = "alternateName")]
    pub alternate_name: Option<String>,
    pub identifier: Option<Value>,
    pub url: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FormalParameterEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "additionalType")]
    pub additional_type: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "valueRequired")]
    pub value_required: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntity {
    #[serde(rename = "@id")]
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "exampleOfWork")]
    pub example_of_work: Option<Value>,
    #[serde(rename = "additionalProperty")]
    pub additional_property: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertyValueEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@additionalType")]
    pub additional_type: Option<String>,
    pub name: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "propertyID")]
    pub property_id: Option<String>,
    #[serde(rename = "valueReference")]
    pub value_reference: Option<String>,
    #[serde(rename = "columnIndex")]
    pub column_index: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreativeWorkEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RootDataEntity {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub type_: String,
    #[serde(rename = "conformsTo")]
    pub conforms_to: Option<Vec<Value>>,
    pub about: Option<Value>,
}