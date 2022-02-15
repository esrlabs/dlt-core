// Copyright (c) 2021 ESR Labs GmbH. All rights reserved.
//
// NOTICE:  All information contained herein is, and remains
// the property of E.S.R.Labs and its suppliers, if any.
// The intellectual and technical concepts contained herein are
// proprietary to E.S.R.Labs and its suppliers and may be covered
// by German and Foreign Patents, patents in process, and are protected
// by trade secret or copyright law.
// Dissemination of this information or reproduction of this material
// is strictly forbidden unless prior written permission is obtained
// from E.S.R.Labs.

//! # official supported DLT service ids

/// Contains all the official service ids with it's u8 representation
/// Maps from the u8 representation to a tuple (service-id-string, explanation)
#[rustfmt::skip]
pub fn service_id_lookup(service_id: u8) -> Option<(&'static str, &'static str)> {
    match service_id {
        0x01 => Some(("set_log_level", "Set the Log Level")),
        0x02 => Some(("set_trace_status", "Enable/Disable Trace Messages")),
        0x03 => Some(("get_log_info", "Returns the LogLevel for registered applications")),
        0x04 => Some(("get_default_log_level", "Returns the LogLevel for wildcards")),
        0x05 => Some(("store_configuration", "Stores the current configuration non volatile")),
        0x06 => Some(("restore_to_factory_default", "Sets the configuration back to default")),
        0x07 => Some(("set_com_interface_status", "SetComInterfaceStatus -- deprecated")),
        0x08 => Some(("set_com_interface_max_bandwidth", "SetComInterfaceMaxBandwidth -- deprecated")),
        0x09 => Some(("set_verbose_mode", "SetVerboseMode -- deprecated")),0x10 => Some(( "set_use_extended_header", "SetUseExtendedHeader -- deprecated")),
        0x0A => Some(("set_message_filtering", "Enable/Disable message filtering")),
        0x0B => Some(("set_timing_packets", "SetTimingPackets -- deprecated")),
        0x0C => Some(("get_local_time", "GetLocalTime -- deprecated")),
        0x0D => Some(("set_use_ecuid", "SetUseECUID -- deprecated")),
        0x0E => Some(("set_use_session_id", "SetUseSessionID -- deprecated")),
        0x0F => Some(("set_use_timestamp", "SetUseTimestamp -- deprecated")),
        0x11 => Some(("set_default_log_level", "Sets the LogLevel for wildcards")),
        0x12 => Some(("set_default_trace_status", "Enable/Disable TraceMessages for wildcards")),
        0x13 => Some(("get_software_version", "Get the ECU software version")),
        0x14 => Some(("message_buffer_overflow", "MessageBufferOverflow -- deprecated")),
        0x15 => Some(("get_default_trace_status", "Get the current TraceLevel for wildcards")),
        0x16 => Some(("get_com_interfacel_status", "GetComInterfacelStatus -- deprecated")),
        0x17 => Some(("get_log_channel_names", "Returns the LogChannel’s name")),
        0x18 => Some(("get_com_interface_max_bandwidth", "GetComInterfaceMaxBandwidth -- deprecated")),
        0x19 => Some(("get_verbose_mode_status", "GetVerboseModeStatus -- deprecated")),
        0x1A => Some(("get_message_filtering_status", "GetMessageFilteringStatus -- deprecated")),
        0x1B => Some(("get_use_ecuid", "GetUseECUID -- deprecated")),
        0x1C => Some(("get_use_session_id", "GetUseSessionID -- deprecated")),
        0x1D => Some(("get_use_timestamp", "GetUseTimestamp -- deprecated")),
        0x1E => Some(("get_use_extended_header", "GetUseExtendedHeader -- deprecated")),
        0x1F => Some(("get_trace_status", "Returns the current TraceStatus")),
        0x20 => Some(("set_log_channel_assignment", "Adds/ Removes the given LogChannel as output path")),
        0x21 => Some(("set_log_channel_threshold", "Sets the filter threshold for the given LogChannel")),
        0x22 => Some(("get_log_channel_threshold", "Returns the current LogLevel for a given LogChannel")),
        0x23 => Some(("buffer_overflow_notification", "Report that a buffer overflow occurred")),
        _ => None,
    }
}
