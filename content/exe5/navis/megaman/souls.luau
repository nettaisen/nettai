--!strict
-- EXE5's souls (Soul Unison), by the name EXE5's code tells them apart by: NaviStats +0x2C, the soul's number in
-- the list of the navis MegaMan unites with (0x0873BBC8's names: 1 ProtoMan, 2 GyroMan, 3 SearchMan, 4 NapalmMan,
-- 5 MagnetMan, 6 Meddy, 7 Colonel, 8 ShadowMan, 9 NumberMan, 10 TomahawkMan, 11 KnightMan, 12 ToadMan; 0 none).
-- A soul is a form of MegaMan's; `souls.of` names the soul a side's navi is in.

local souls = {}

export type Soul =
    "proto"
    | "gyro"
    | "search"
    | "napalm"
    | "magnet"
    | "meddy"
    | "colonel"
    | "shadow"
    | "number"
    | "tomahawk"
    | "knight"
    | "toad"

-- The soul forms by their ids.
local by_form: { [string]: Soul } = {
    ["protosoul"] = "proto",
    ["gyrosoul"] = "gyro",
    ["searchsoul"] = "search",
    ["napalmsoul"] = "napalm",
    ["magnetsoul"] = "magnet",
    ["meddysoul"] = "meddy",
    ["colonelsoul"] = "colonel",
    ["shadowsoul"] = "shadow",
    ["numbersoul"] = "number",
    ["tomahawksoul"] = "tomahawk",
    ["knightsoul"] = "knight",
    ["toadsoul"] = "toad",
}

-- The soul `navi`'s side is in, nil for none.
function souls.of(navi: Object): Soul?
    return by_form[battle.navi(navi.alliance).form.id]
end

return souls
