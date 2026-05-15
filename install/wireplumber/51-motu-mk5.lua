-- MOTU UltraLite mk5 — WirePlumber device configuration
-- Node naming/routing is handled by the PipeWire motu-router module.

table.insert(alsa_monitor.rules, {
  matches = {
    {
      { "device.name", "matches", "alsa_card.usb-MOTU_UltraLite*" },
    },
  },
  apply_properties = {
    ["device.description"] = "MOTU UltraLite mk5",
    ["device.nick"] = "MOTU mk5",
    ["device.profile-set"] = "motu-ultralite-mk5.conf",
    ["device.profile"] = "pro-audio",
  },
})
