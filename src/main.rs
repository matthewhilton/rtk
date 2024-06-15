use std::{fmt::Display, fs::File, io::Read};

/**
 * The header to detect an RTCM3 message.
 * In binary, is:
 * 11010011
 */
const EXPECTED_FIRST_BYTE: u8 = 0xD3;

struct RTCM3Message {
    raw: Vec<u8>
}

#[derive(strum_macros::Display)]
enum MessageType {
    #[strum(to_string = "Unknown<value: {val}>")]
    Unknown { val: u16 },

    StationaryRTKReferenceStationARPWithAntennaHeight,
    AntennaDescriptorAndSerialNumber,
    ReceiverWithAntennaDescriptors,

    GPSExtendedL1AndL2RTKObservables,
    GPSMSM7,

    BeiDouEphemeris,
    BeiDouMSM7,

    GalileoEphemeris,
    GalileoMSM7,

    GLONASSMSM7,
    GLONASSL1AndL2CodePhaseBiases,

    QZSSMSM7,
}

trait Message {
    fn get_type(&self) -> MessageType;
}

impl Message for RTCM3Message {
    fn get_type(&self) -> MessageType {
        let msgtype = (self.raw[0] as u16) << 4 | (self.raw[1] as u16) >> 4;

        // Based off https://www.use-snip.com/kb/knowledge-base/rtcm-3-message-list/
        match msgtype {
            1004 => MessageType::GPSExtendedL1AndL2RTKObservables,
            1042 => MessageType::BeiDouEphemeris,
            1046 => MessageType::GalileoEphemeris,
            1127 => MessageType::BeiDouMSM7,
            1077 => MessageType::GPSMSM7,
            1087 => MessageType::GLONASSMSM7,
            1117 => MessageType::QZSSMSM7,
            1097 => MessageType::GalileoMSM7,
            1006 => MessageType::StationaryRTKReferenceStationARPWithAntennaHeight,
            1008 => MessageType::AntennaDescriptorAndSerialNumber,
            1033 => MessageType::ReceiverWithAntennaDescriptors,
            1230 => MessageType::GLONASSL1AndL2CodePhaseBiases,
            _ => MessageType::Unknown {
                val: msgtype
            }
        }
    }
}

fn main() {
    let messages = parse_rtcm3().unwrap();

    for msg in messages {
        println!("Message - type: {}", msg.get_type())
    }
}

fn parse_rtcm3() -> Result<Vec<RTCM3Message>, String> {
    let mut f = File::open("sample_data")
        .map_err(|_| "Could not open file")?;

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).map_err(|_| "Error reading file")?;

    let mut messages = Vec::new();

    let mut offset = 0;
    while offset < buffer.len() {
        let data = &buffer[offset..];
        let byte1 = data[0];

        // Check if this is a RTCM3 start byte marker, skip if not.
        if byte1 != EXPECTED_FIRST_BYTE {
            offset += 1;
            continue;
        }

        // Then combine the next two bytes.
        // The first six bits are zero reserved, but the last 10 are the length
        // of the frame.
        // this makes 16 in total, so we just assume the first six are zero.
        let byte2 = data[1];
        let byte3 = data[2];
        let length = (((byte2 as u16) << 8) | byte3 as u16) as usize;

        // Ignore incomplete end of file frames.
        if data.len() < length + 6 {
            offset += 1;
            continue;
        }

        // Get the CRC info and calculate the crc.
        // It is good if the calculated CRC is zero.
        let crc = &data[length + 3..length + 6];
        let fulldata = &data[0..length + 6];
        let calculated_crc = crc24q_new(fulldata);

        // Bad checksum - skip.
        if calculated_crc != 0 {
            offset += 1;
            continue;
        }

        // Now read the actual message.
        let msg = &data[3..length + 3];

        messages.push(RTCM3Message {
            raw: msg.to_vec()
        });

        // The type is the first 12 bits. So take the first byte (8 bits) and the last 4 bits of the second byte.
        //let msgtype = (msg[0] as u16) << 4 | (msg[1] as u16) >> 4;

        println!("Frame - length: {} - crc: {:#x} {:#x} {:#x} - calculated crc: {}", length, crc[0], crc[1], crc[2], calculated_crc);

        // Move the offset forward.
        // The total length of the frame is:
        // 1 byte - header
        // 2 bytes - length info
        // n bytes - the length of the frame
        // 4 bytes - type + crc
        offset += length as usize + 7;
    }

    println!("Done!");

    Ok(messages)
}

fn crc24q_new(data: &[u8]) -> u32 {
    let mut crc: u32 = 0;
    let poly = 0x1864CFB;

    for octet in data {
        crc ^= (*octet as u32) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x1000000 != 0 {
                crc ^= poly;
            }
        }
    }

    return crc & 0xFFFFFF;
}