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
    ["device.profile"] = "all-io",
  },
})

-- Output nodes
local outputs = {
  { suffix = "out-main",   desc = "MOTU Main 1/2",     priority = 2007 },
  { suffix = "out-line34", desc = "MOTU Line 3/4",     priority = 2006 },
  { suffix = "out-line56", desc = "MOTU Line 5/6",     priority = 2005 },
  { suffix = "out-line78", desc = "MOTU Line 7/8",     priority = 2004 },
  { suffix = "out-line910",desc = "MOTU Line 9/10",    priority = 2003 },
  { suffix = "out-phones", desc = "MOTU Phones",       priority = 2002 },
  { suffix = "out-spdif",  desc = "MOTU S/PDIF Out",   priority = 2001 },
}

for _, o in ipairs(outputs) do
  table.insert(alsa_monitor.rules, {
    matches = {
      {
        { "node.name", "matches", "alsa_output.usb-MOTU_UltraLite*." .. o.suffix },
      },
    },
    apply_properties = {
      ["node.description"] = o.desc,
      ["node.nick"] = o.desc,
      ["priority.session"] = o.priority,
      ["priority.driver"] = o.priority,
    },
  })
end

-- Input nodes
local inputs = {
  { suffix = "in-mic12",  desc = "MOTU Mic/Line 1/2", priority = 2005 },
  { suffix = "in-line34", desc = "MOTU Line In 3/4",  priority = 2004 },
  { suffix = "in-line56", desc = "MOTU Line In 5/6",  priority = 2003 },
  { suffix = "in-line78", desc = "MOTU Line In 7/8",  priority = 2002 },
  { suffix = "in-spdif",  desc = "MOTU S/PDIF In",    priority = 2001 },
}

for _, i in ipairs(inputs) do
  table.insert(alsa_monitor.rules, {
    matches = {
      {
        { "node.name", "matches", "alsa_input.usb-MOTU_UltraLite*." .. i.suffix },
      },
    },
    apply_properties = {
      ["node.description"] = i.desc,
      ["node.nick"] = i.desc,
      ["priority.session"] = i.priority,
      ["priority.driver"] = i.priority,
    },
  })
end
