mod netmd;

use crate::netmd::base::NetMD;
use crate::netmd::interface::NetMDInterface;

use std::thread;
use std::time::Duration;

fn main() {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();

        let new_device = match device.open() {
            Ok(device) => device,
            Err(_) => continue,
        };

        println!("Connected to Bus {:03} Device {:03} VID: {:04x}, PID: {:04x}, {:?}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id(),
            new_device.read_product_string_ascii(&device_desc));

        let player = NetMD::new(new_device, device_desc).unwrap();
        let player_controller = NetMDInterface::new(player);

        println!(
            "Player Model: {}",
            player_controller.net_md_device.device_name().unwrap()
        );

        player_controller.play();

        thread::sleep(Duration::from_secs(1));

        player_controller.stop();

        /*
        let mut request: [u8; 19] = [0x00, 0x18, 0x06, 0x02, 0x20, 0x18,
                                     0x01, 0x00, 0x00, 0x30, 0x00, 0xa,
                                     0x00, 0xff, 0x00, 0x00, 0x00, 0x00,
                                     0x00];
        request[4] = 0x75;
        let test_result = player.send_command(&request);

        match test_result {
            Ok(_) => println!("Successfully sent command! Response: {:?}", test_result),
            Err(error) => println!("Command failed! Error: {}", error)
        }
        */
    }
}