use std::fmt::Display;
use clap::ValueEnum;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum VmType {
    #[serde(rename = "t2.nano")]
    T2Nano,
    #[serde(rename = "t2.micro")]
    T2Micro,
    #[serde(rename = "t2.small")]
    T2Small,
    #[serde(rename = "t2.medium")]
    T2Medium,
    #[serde(rename = "t2.large")]
    T2Large,
    #[serde(rename = "t2.xlarge")]
    T2XLarge,
    #[serde(rename = "t2.2xlarge")]
    T22XLarge,
    #[serde(rename = "t3a.nano")]
    T3ANano,
    #[serde(rename = "t3a.micro")]
    T3AMicro,
    #[serde(rename = "t3a.small")]
    T3ASmall,
    #[serde(rename = "t3a.medium")]
    T3AMedium,
    #[serde(rename = "t3a.large")]
    T3ALarge,
    #[serde(rename = "t3a.xlarge")]
    T3AXLarge,
    #[serde(rename = "t3a.2xlarge")]
    T3A2XLarge,
    #[serde(rename = "t3.nano")]
    T3Nano,
    #[serde(rename = "t3.micro")]
    T3Micro,
    #[serde(rename = "t3.small")]
    T3Small,
    #[serde(rename = "t3.medium")]
    T3Medium,
    #[serde(rename = "t3.large")]
    T3Large,
    #[serde(rename = "t3.xlarge")]
    T3XLarge,
    #[serde(rename = "t3.2xlarge")]
    T32XLarge,
    #[serde(rename = "t4g.nano")]
    T4GNano,
    #[serde(rename = "t4g.micro")]
    T4GMicro,
    #[serde(rename = "t4g.small")]
    T4GSmall,
    #[serde(rename = "t4g.medium")]
    T4GMedium,
    #[serde(rename = "t4g.large")]
    T4GLarge,
    #[serde(rename = "t4g.xlarge")]
    T4GXLarge,
    #[serde(rename = "t4g.2xlarge")]
    T4G2XLarge,
    #[serde(rename = "m4.large")]
    M4Large,
    #[serde(rename = "m4.xlarge")]
    M4XLarge,
    #[serde(rename = "m4.2xlarge")]
    M42XLarge,
    #[serde(rename = "m4.4xlarge")]
    M44XLarge,
    #[serde(rename = "m5a.large")]
    M5ALarge,
    #[serde(rename = "m5a.xlarge")]
    M5AXLarge,
    #[serde(rename = "m5a.2xlarge")]
    M5A2XLarge,
    #[serde(rename = "m5a.4xlarge")]
    M5A4XLarge,
    #[serde(rename = "m5ad.large")]
    M5ADLarge,
    #[serde(rename = "m5ad.xlarge")]
    M5ADXLarge,
    #[serde(rename = "m5ad.2xlarge")]
    M5AD2XLarge,
    #[serde(rename = "m5ad.4xlarge")]
    M5AD4XLarge,
    #[serde(rename = "m5zn.large")]
    M5ZNLarge,
    #[serde(rename = "m5zn.xlarge")]
    M5ZNXLarge,
    #[serde(rename = "m5zn.2xlarge")]
    M5ZN2XLarge,
    #[serde(rename = "m5zn.3xlarge")]
    M5ZN3XLarge,
    #[serde(rename = "m5n.large")]
    M5NLarge,
    #[serde(rename = "m5n.xlarge")]
    M5NXLarge,
    #[serde(rename = "m5n.2xlarge")]
    M5N2XLarge,
    #[serde(rename = "m5n.4xlarge")]
    M5N4XLarge,
    #[serde(rename = "m5dn.large")]
    M5DNLarge,
    #[serde(rename = "m5dn.xlarge")]
    M5DNXLarge,
    #[serde(rename = "m5dn.2xlarge")]
    M5DN2XLarge,
    #[serde(rename = "m5dn.4xlarge")]
    M5DN4XLarge,
    #[serde(rename = "m6a.large")]
    M6ALarge,
    #[serde(rename = "m6a.xlarge")]
    M6AXLarge,
    #[serde(rename = "m6a.2xlarge")]
    M6A2XLarge,
    #[serde(rename = "m6a.4xlarge")]
    M6A4XLarge,
    #[serde(rename = "m6in.large")]
    M6INLarge,
    #[serde(rename = "m6in.xlarge")]
    M6INXLarge,
    #[serde(rename = "m6in.2xlarge")]
    M6IN2XLarge,
    #[serde(rename = "m6in.4xlarge")]
    M6IN4XLarge,
    #[serde(rename = "m6idn.large")]
    M6IDNLarge,
    #[serde(rename = "m6idn.xlarge")]
    M6IDNXLarge,
    #[serde(rename = "m6idn.2xlarge")]
    M6IDN2XLarge,
    #[serde(rename = "m6idn.4xlarge")]
    M6IDN4XLarge,
    #[serde(rename = "m6i.large")]
    M6ILarge,
    #[serde(rename = "m6ix.large")]
    M6IXLarge,
    #[serde(rename = "m6i.2xlarge")]
    M6I2XLarge,
    #[serde(rename = "m6i.4xlarge")]
    M6I4XLarge,
    #[serde(rename = "m6id.large")]
    M6IDLarge,
    #[serde(rename = "m6id.xlarge")]
    M6IDXLarge,
    #[serde(rename = "m6id.2xlarge")]
    M6ID2XLarge,
    #[serde(rename = "m6g.medium")]
    M6GMedium,
    #[serde(rename = "m6g.large")]
    M6GLarge,
    #[serde(rename = "m6g.xlarge")]
    M6GXLarge,
    #[serde(rename = "m6g.2xlarge")]
    M6G2XLarge,
    #[serde(rename = "m6g.4xlarge")]
    M6G4XLarge,
    #[serde(rename = "m6gd.medium")]
    M6GDMedium,
    #[serde(rename = "m6gd.large")]
    M6GDLarge,
    #[serde(rename = "m6gd.xlarge")]
    M6GDXLarge,
    #[serde(rename = "m6gd.2xlarge")]
    M6GD2XLarge,
    #[serde(rename = "m6gd.4xlarge")]
    M6GD4XLarge,
    #[serde(rename = "m7a.medium")]
    M7AMedium,
    #[serde(rename = "m7a.large")]
    M7ALarge,
    #[serde(rename = "m7a.xlarge")]
    M7AXLarge,
    #[serde(rename = "m7a.2xlarge")]
    M7A2XLarge,
    #[serde(rename = "m7a.4xlarge")]
    M7A4XLarge,
    #[serde(rename = "m7i-flex.large")]
    M7IFlexLarge,
    #[serde(rename = "m7i-flex.xlarge")]
    M7IFlexXLarge,
    #[serde(rename = "m7i-flex.2xlarge")]
    M7IFlex2XLarge,
    #[serde(rename = "m7i-flex.4xlarge")]
    M7IFlex4XLarge,
    #[serde(rename = "m7i.large")]
    M7ILarge,
    #[serde(rename = "m7i.xlarge")]
    M7IXLarge,
    #[serde(rename = "m7i.2xlarge")]
    M7I2XLarge,
    #[serde(rename = "m7i.4xlarge")]
    M7I4XLarge,
    #[serde(rename = "m7g.medium")]
    M7GMedium,
    #[serde(rename = "m7g.large")]
    M7GLarge,
    #[serde(rename = "m7g.xlarge")]
    M7GXLarge,
    #[serde(rename = "m7g.2xlarge")]
    M7G2XLarge,
    #[serde(rename = "m7g.4xlarge")]
    M7G4XLarge,
    #[serde(rename = "m7gd.medium")]
    M7GDMedium,
    #[serde(rename = "m7gd.large")]
    M7GDLarge,
    #[serde(rename = "m7gd.xlarge")]
    M7GDXLarge,
    #[serde(rename = "m7gd.2xlarge")]
    M7GD2XLarge,
    #[serde(rename = "m7gd.4xlarge")]
    M7GD4XLarge
}

impl Display for VmType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::T2Nano => write!(f, "t2.nano"), 
            Self::T2Micro => write!(f, "t2.micro"),
            Self::T2Small => write!(f, "t2.small"),
            Self::T2Medium => write!(f, "t2.medium"),
            Self::T2Large => write!(f, "t2.large"),
            Self::T2XLarge => write!(f, "t2.xlarge"),
            Self::T22XLarge=> write!(f, "t2.2xlarge"),
            Self::T3ANano => write!(f, "t3a.nano"),
            Self::T3AMicro => write!(f, "t3a.micro"),
            Self::T3ASmall => write!(f, "t3a.small"),
            Self::T3AMedium => write!(f, "t3a.medium"),
            Self::T3ALarge => write!(f, "t3a.large"),
            Self::T3AXLarge => write!(f, "t3a.xlarge"),
            Self::T3A2XLarge => write!(f, "t3a.2xlarge"),
            Self::T3Nano => write!(f, "t3.nano"),
            Self::T3Micro => write!(f, "t3.micro"),
            Self::T3Small => write!(f, "t3.small"),
            Self::T3Medium => write!(f, "t3.medium"),
            Self::T3Large => write!(f, "t3.large"),
            Self::T3XLarge => write!(f, "t3.xlarge"),
            Self::T32XLarge => write!(f, "t3.2xlarge"),
            Self::T4GNano => write!(f, "t4g.nano"),
            Self::T4GMicro => write!(f, "t4g.micro"),
            Self::T4GSmall => write!(f, "t4g.small"),
            Self::T4GMedium => write!(f, "t4g.medium"),
            Self::T4GLarge => write!(f, "t4g.large"),
            Self::T4GXLarge => write!(f, "t4g.xlarge"),
            Self::T4G2XLarge => write!(f, "t4g.2xlarge"),
            Self::M4Large => write!(f, "m4.large"),
            Self::M4XLarge => write!(f, "m4.xlarge"),
            Self::M42XLarge => write!(f, "m4.2xlarge"),
            Self::M44XLarge => write!(f, "m4.4xlarge"),
            Self::M5ALarge => write!(f, "m5a.large"),
            Self::M5AXLarge => write!(f, "m5a.xlarge"),
            Self::M5A2XLarge => write!(f, "m5a.2xlarge"),
            Self::M5A4XLarge => write!(f, "m5a.4xlarge"),
            Self::M5ADLarge => write!(f, "m5ad.large"),
            Self::M5ADXLarge => write!(f, "m5ad.xlarge"),
            Self::M5AD2XLarge => write!(f, "m5ad.2xlarge"),
            Self::M5AD4XLarge => write!(f, "m5ad.4xlarge"),
            Self::M5ZNLarge => write!(f, "m5zn.large"),
            Self::M5ZNXLarge => write!(f, "m5zn.xlarge"),
            Self::M5ZN2XLarge => write!(f, "m5zn.2xlarge"),
            Self::M5ZN3XLarge => write!(f, "m5zn.3xlarge"),
            Self::M5NLarge => write!(f, "m5n.large"),
            Self::M5NXLarge => write!(f, "m5n.xlarge"),
            Self::M5N2XLarge => write!(f, "m5n.2xlarge"),
            Self::M5N4XLarge => write!(f, "m5n.4xlarge"),
            Self::M5DNLarge => write!(f, "m5dn.large"),
            Self::M5DNXLarge => write!(f, "m5dn.xlarge"),
            Self::M5DN2XLarge => write!(f, "m5dn.2xlarge"),
            Self::M5DN4XLarge => write!(f, "m5dn.4xlarge"),
            Self::M6ALarge => write!(f, "m6a.large"),
            Self::M6AXLarge => write!(f, "m6a.xlarge"),
            Self::M6A2XLarge => write!(f, "m6a.2xlarge"),
            Self::M6A4XLarge => write!(f, "m6a.4xlarge"),
            Self::M6INLarge => write!(f, "m6in.larage"),
            Self::M6INXLarge => write!(f, "m6in.xlarge"),
            Self::M6IN2XLarge => write!(f, "m6in.2xlarge"),
            Self::M6IN4XLarge => write!(f, "m6in.4xlarge"),
            Self::M6IDNLarge => write!(f, "m6idn.large"),
            Self::M6IDNXLarge => write!(f, "m6idn.xlarge"),
            Self::M6IDN2XLarge => write!(f, "m6idn.2xlarge"),
            Self::M6IDN4XLarge => write!(f, "m6idn.4xlarge"),
            Self::M6ILarge => write!(f, "m6i.large"),
            Self::M6IXLarge => write!(f, "m6ix.large"),
            Self::M6I2XLarge => write!(f, "m6i.2xlarge"),
            Self::M6I4XLarge => write!(f, "m6i.4xlarge"),
            Self::M6IDLarge => write!(f, "m6id.large"),
            Self::M6IDXLarge => write!(f, "m6id.xlarge"),
            Self::M6ID2XLarge => write!(f, "m6id.2xlarge"),
            Self::M6GMedium => write!(f, "m6g.medium"),
            Self::M6GLarge => write!(f, "m6g.large"),
            Self::M6GXLarge => write!(f, "m6g.xlarge"),
            Self::M6G2XLarge => write!(f, "m6g.2xlarge"),
            Self::M6G4XLarge => write!(f, "m6g.4xlarge"),
            Self::M6GDMedium => write!(f, "m6gd.medium"),
            Self::M6GDLarge => write!(f, "m6gd.large"),
            Self::M6GDXLarge => write!(f, "m6gd.xlarge"),
            Self::M6GD2XLarge => write!(f, "m6gd.2xlarge"),
            Self::M6GD4XLarge => write!(f, "m6gd.4xlarge"),
            Self::M7AMedium => write!(f, "m7a.medium"),
            Self::M7ALarge => write!(f, "m7a.large"),
            Self::M7AXLarge => write!(f, "m7a.xlarge"),
            Self::M7A2XLarge => write!(f, "m7a.2xlarge"),
            Self::M7A4XLarge => write!(f, "m7a.4xlarge"),
            Self::M7IFlexLarge => write!(f, "m7i-flex.large"),
            Self::M7IFlexXLarge => write!(f, "m7i-flex.xlarge"),
            Self::M7IFlex2XLarge => write!(f, "m7i-flex.2xlarge"),
            Self::M7IFlex4XLarge => write!(f, "m7i-flex.4xlarge"),
            Self::M7ILarge => write!(f, "m7i.large"),
            Self::M7IXLarge => write!(f, "m7i.xlarge"),
            Self::M7I2XLarge => write!(f, "m7i.2xlarge"),
            Self::M7I4XLarge => write!(f, "m7i.4xlarge"),
            Self::M7GMedium => write!(f, "m7g.medium"),
            Self::M7GLarge => write!(f, "m7g.large"),
            Self::M7GXLarge => write!(f, "m7g.xlarge"),
            Self::M7G2XLarge => write!(f, "m7g.2xlarge"),
            Self::M7G4XLarge => write!(f, "m7g.4xlarge"),
            Self::M7GDMedium => write!(f, "m7gd.medium"),
            Self::M7GDLarge => write!(f, "m7gd.large"),
            Self::M7GDXLarge => write!(f, "m7gd.xlarge"),
            Self::M7GD2XLarge => write!(f, "m7gd.2xlarge"),
            Self::M7GD4XLarge => write!(f, "m7gd.4xlarge"),
        }
    }
}

use std::str::FromStr;
use std::io;

impl FromStr for VmType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "t2.nano" => Ok(VmType::T2Nano),
            "t2.micro" => Ok(VmType::T2Micro),
            "t2.small" => Ok(VmType::T2Small),
            "t2.medium" => Ok(VmType::T2Medium),
            "t2.large" => Ok(VmType::T2Large),
            "t2.xlarge" => Ok(VmType::T2XLarge),
            "t2.2xlarge" => Ok(VmType::T22XLarge),
            "t3a.nano" => Ok(VmType::T3ANano),
            "t3a.micro" => Ok(VmType::T3AMicro),
            "t3a.small" => Ok(VmType::T3ASmall),
            "t3a.medium" => Ok(VmType::T3AMedium),
            "t3a.large" => Ok(VmType::T3ALarge),
            "t3a.xlarge" => Ok(VmType::T3AXLarge),
            "t3a.2xlarge" => Ok(VmType::T3A2XLarge),
            "t3.nano" => Ok(VmType::T3Nano),
            "t3.micro" => Ok(VmType::T3Micro),
            "t3.small" => Ok(VmType::T3Small),
            "t3.medium" => Ok(VmType::T3Medium),
            "t3.large" => Ok(VmType::T3Large),
            "t3.xlarge" => Ok(VmType::T3XLarge),
            "t3.2xlarge" => Ok(VmType::T32XLarge),
            "t4g.nano" => Ok(VmType::T4GNano),
            "t4g.micro" => Ok(VmType::T4GMicro),
            "t4g.small" => Ok(VmType::T4GSmall),
            "t4g.medium" => Ok(VmType::T4GMedium),
            "t4g.large" => Ok(VmType::T4GLarge),
            "t4g.xlarge" => Ok(VmType::T4GXLarge),
            "t4g.2xlarge" => Ok(VmType::T4G2XLarge),
            "m4.large" => Ok(VmType::M4Large),
            "m4.xlarge" => Ok(VmType::M4XLarge),
            "m4.2xlarge" => Ok(VmType::M42XLarge),
            "m4.4xlarge" => Ok(VmType::M44XLarge),
            "m5a.large" => Ok(VmType::M5ALarge),
            "m5a.xlarge" => Ok(VmType::M5AXLarge),
            "m5a.2xlarge" => Ok(VmType::M5A2XLarge),
            "m5a.4xlarge" => Ok(VmType::M5A4XLarge),
            "m5ad.large" => Ok(VmType::M5ADLarge),
            "m5ad.xlarge" => Ok(VmType::M5ADXLarge),
            "m5ad.2xlarge" => Ok(VmType::M5AD2XLarge),
            "m5ad.4xlarge" => Ok(VmType::M5AD4XLarge),
            "m5zn.large" => Ok(VmType::M5ZNLarge),
            "m5zn.xlarge" => Ok(VmType::M5ZNXLarge),
            "m5zn.2xlarge" => Ok(VmType::M5ZN2XLarge),
            "m5zn.3xlarge" => Ok(VmType::M5ZN3XLarge),
            "m5n.large" => Ok(VmType::M5NLarge),
            "m5n.xlarge" => Ok(VmType::M5NXLarge),
            "m5n.2xlarge" => Ok(VmType::M5N2XLarge),
            "m5n.4xlarge" => Ok(VmType::M5N4XLarge),
            "m5dn.large" => Ok(VmType::M5DNLarge),
            "m5dn.xlarge" => Ok(VmType::M5DNXLarge),
            "m5dn.2xlarge" => Ok(VmType::M5DN2XLarge),
            "m5dn.4xlarge" => Ok(VmType::M5DN4XLarge),
            "m6a.large" => Ok(VmType::M6ALarge),
            "m6a.xlarge" => Ok(VmType::M6AXLarge),
            "m6a.2xlarge" => Ok(VmType::M6A2XLarge),
            "m6a.4xlarge" => Ok(VmType::M6A4XLarge),
            "m6in.large" => Ok(VmType::M6INLarge),
            "m6in.xlarge" => Ok(VmType::M6INXLarge),
            "m6in.2xlarge" => Ok(VmType::M6IN2XLarge),
            "m6in.4xlarge" => Ok(VmType::M6IN4XLarge),
            "m6idn.large" => Ok(VmType::M6IDNLarge),
            "m6idn.xlarge" => Ok(VmType::M6IDNXLarge),
            "m6idn.2xlarge" => Ok(VmType::M6IDN2XLarge),
            "m6idn.4xlarge" => Ok(VmType::M6IDN4XLarge),
            "m6i.large" => Ok(VmType::M6ILarge),
            "m6ix.large" => Ok(VmType::M6IXLarge),
            "m6i.2xlarge" => Ok(VmType::M6I2XLarge),
            "m6i.4xlarge" => Ok(VmType::M6I4XLarge),
            "m6id.large" => Ok(VmType::M6IDLarge),
            "m6id.xlarge" => Ok(VmType::M6IDXLarge),
            "m6id.2xlarge" => Ok(VmType::M6ID2XLarge),
            "m6g.medium" => Ok(VmType::M6GMedium),
            "m6g.large" => Ok(VmType::M6GLarge),
            "m6g.xlarge" => Ok(VmType::M6GXLarge),
            "m6g.2xlarge" => Ok(VmType::M6G2XLarge),
            "m6g.4xlarge" => Ok(VmType::M6G4XLarge),
            "m6gd.medium" => Ok(VmType::M6GDMedium),
            "m6gd.large" => Ok(VmType::M6GDLarge),
            "m6gd.xlarge" => Ok(VmType::M6GDXLarge),
            "m6gd.2xlarge" => Ok(VmType::M6GD2XLarge),
            "m6gd.4xlarge" => Ok(VmType::M6GD4XLarge),
            "m7a.medium" => Ok(VmType::M7AMedium),
            "m7a.large" => Ok(VmType::M7ALarge),
            "m7a.xlarge" => Ok(VmType::M7AXLarge),
            "m7a.2xlarge" => Ok(VmType::M7A2XLarge),
            "m7a.4xlarge" => Ok(VmType::M7A4XLarge),
            "m7i-flex.large" => Ok(VmType::M7IFlexLarge),
            "m7i-flex.xlarge" => Ok(VmType::M7IFlexXLarge),
            "m7i-flex.2xlarge" => Ok(VmType::M7IFlex2XLarge),
            "m7i-flex.4xlarge" => Ok(VmType::M7IFlex4XLarge),
            "m7i.large" => Ok(VmType::M7ILarge),
            "m7i.xlarge" => Ok(VmType::M7IXLarge),
            "m7i.2xlarge" => Ok(VmType::M7I2XLarge),
            "m7i.4xlarge" => Ok(VmType::M7I4XLarge),
            "m7g.medium" => Ok(VmType::M7GMedium),
            "m7g.large" => Ok(VmType::M7GLarge),
            "m7g.xlarge" => Ok(VmType::M7GXLarge),
            "m7g.2xlarge" => Ok(VmType::M7G2XLarge),
            "m7g.4xlarge" => Ok(VmType::M7G4XLarge),
            "m7gd.medium" => Ok(VmType::M7GDMedium),
            "m7gd.large" => Ok(VmType::M7GDLarge),
            "m7gd.xlarge" => Ok(VmType::M7GDXLarge),
            "m7gd.2xlarge" => Ok(VmType::M7GD2XLarge),
            "m7gd.4xlarge" => Ok(VmType::M7GD4XLarge),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid VmType")),
        }
    }
}
