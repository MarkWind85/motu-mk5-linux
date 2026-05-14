-- MOTU UltraLite mk5 — dynamic WirePlumber configuration

-- Configure the ALSA card device to use our custom profile set
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
    ["device.profile"] = "output:out-main",
  },
})

-- Configure the capture (input) node
table.insert(alsa_monitor.rules, {
  matches = {
    {
      { "node.name", "matches", "alsa_input.usb-MOTU_UltraLite*" },
    },
  },
  apply_properties = {
    ["node.description"] = "MOTU mk5 Input",
    ["node.nick"] = "MOTU Input",
    ["priority.session"] = 2000,
    ["priority.driver"] = 2000,
  },
})

-- Configure the playback (output) node
table.insert(alsa_monitor.rules, {
  matches = {
    {
      { "node.name", "matches", "alsa_output.usb-MOTU_UltraLite*" },
    },
  },
  apply_properties = {
    ["node.description"] = "MOTU mk5 Output",
    ["node.nick"] = "MOTU Output",
    ["priority.session"] = 2000,
    ["priority.driver"] = 2000,
  },
})
