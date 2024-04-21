use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub title: Title,
    pub grid: Grid,
    pub tooltip: Tooltip,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub series: Vec<Series>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub text: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tooltip {
    // pub trigger: String,
    // pub axis_pointer: AxisPointer,
    pub formatter: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Legend {
    pub data: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grid {
    // pub left: String,
    // pub right: String,
    // pub bottom: String,
    pub contain_label: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toolbox {
    pub feature: Feature,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub save_as_image: SaveAsImage,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAsImage {}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Axis {
    pub r#type: String,
    pub name: String,
    pub max: u64,
    // pub min: String,
    pub log_base: u64,
    // pub min: u64,
    // pub axis_pointer: AxisPointer,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    rename_all_fields = "camelCase",
    rename_all = "camelCase",
    tag = "type"
)]
pub enum Series {
    Line {
        step: String,
        data: Vec<[u64; 2]>,
        // area_style: AreaStyle,
    },
    Scatter {
        data: Vec<[u64; 2]>,
        // tooltip: Tooltip,
    },
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaStyle {
    pub opacity: f32,
    pub origin: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AxisPointer {
    pub show: bool,
    pub r#type: String,
    pub snap: bool,
    pub label: Label,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub precision: String,
}
