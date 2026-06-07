#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Order type classification for perpetual and advanced order flows.
/// Spot markets use a plain `OrderId` (`u64`) and do not embed order type in the id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OrderIdType {
    Limit = 1,
    Market = 2,
    Trigger = 3,
    Scale = 4,
    StopLimit = 5,
    StopMarket = 6,
    TWAP = 7,
}

impl OrderIdType {
    pub fn is_limit(&self) -> bool {
        matches!(self, OrderIdType::Limit)
    }

    pub fn is_market(&self) -> bool {
        matches!(self, OrderIdType::Market)
    }

    pub fn is_trigger(&self) -> bool {
        matches!(self, OrderIdType::Trigger)
    }

    pub fn is_scale(&self) -> bool {
        matches!(self, OrderIdType::Scale)
    }

    pub fn is_stop_limit(&self) -> bool {
        matches!(self, OrderIdType::StopLimit)
    }

    pub fn is_stop_market(&self) -> bool {
        matches!(self, OrderIdType::StopMarket)
    }

    pub fn is_twap(&self) -> bool {
        matches!(self, OrderIdType::TWAP)
    }

    pub fn is_spot_compatible(&self) -> bool {
        matches!(
            self,
            OrderIdType::Limit | OrderIdType::Market | OrderIdType::Trigger
        )
    }

    pub fn is_perp_compatible(&self) -> bool {
        matches!(
            self,
            OrderIdType::Limit
                | OrderIdType::Market
                | OrderIdType::Scale
                | OrderIdType::StopLimit
                | OrderIdType::StopMarket
                | OrderIdType::TWAP
        )
    }
}

impl std::fmt::Display for OrderIdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderIdType::Limit => write!(f, "Limit"),
            OrderIdType::Market => write!(f, "Market"),
            OrderIdType::Trigger => write!(f, "Trigger"),
            OrderIdType::Scale => write!(f, "Scale"),
            OrderIdType::StopLimit => write!(f, "StopLimit"),
            OrderIdType::StopMarket => write!(f, "StopMarket"),
            OrderIdType::TWAP => write!(f, "TWAP"),
        }
    }
}

impl From<u8> for OrderIdType {
    fn from(value: u8) -> Self {
        match value {
            1 => OrderIdType::Limit,
            2 => OrderIdType::Market,
            3 => OrderIdType::Trigger,
            4 => OrderIdType::Scale,
            5 => OrderIdType::StopLimit,
            6 => OrderIdType::StopMarket,
            7 => OrderIdType::TWAP,
            _ => panic!("Invalid OrderIdType value: {}", value),
        }
    }
}

impl From<OrderIdType> for u8 {
    fn from(order_type: OrderIdType) -> Self {
        order_type as u8
    }
}
