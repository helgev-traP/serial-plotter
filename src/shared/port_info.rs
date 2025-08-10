#[derive(Clone, Debug)]
pub struct PortsInfo {
    pub available_ports: Vec<String>,
    pub available_baud_rates: Vec<u32>,
    pub selected_port: Option<String>,
    pub baud_rate: u32,
}

#[allow(clippy::new_without_default)]
impl PortsInfo {
    pub fn new() -> Self {
        let available_ports = serialport::available_ports()
            .map(|ports| {
                ports
                    .into_iter()
                    .map(|port| port.port_name)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // let available_baud_rates = BAUD_RATES
        //     .iter()
        //     .filter(|&&rate| serialport::new(&port_name, rate).open().is_ok())
        //     .cloned()
        //     .collect();

        let available_baud_rates = BAUD_RATES.to_vec();

        Self {
            available_ports,
            available_baud_rates,
            selected_port: None,
            baud_rate: 9600,
        }
    }
}

const BAUD_RATES: &[u32] = &[
    300, 600, 750, 1_200, 2_400, 4_800, 9_600, 19_200, 31_250, 38_400, 57_600, 74_880, 115_200,
    230_400, 250_000, 460_800, 500_000, 921_600, 1_000_000, 2_000_000,
];
