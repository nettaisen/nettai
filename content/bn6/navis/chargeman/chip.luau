--!strict
-- ChargeMan's own chip, VolcChrg (the original's `sub_80F1334`, his action
-- table's entry 0xA): he blows his stack (30 ticks), and 19 ticks in his
-- volcano throws five rocks (navis/chargeman/volcano_rock): at the enemy
-- navis' panels first, then at the other side's empty panels in a shuffled
-- order, then at the column off the far edge; 30 ticks of recovery. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local link = require("../../lib/link_chips")
local panels = require("../../lib/panels")
local volcano_rock = require("./volcano_rock")

export type State = { timer: number }

local STATE = { timer = "u16" }

local ERUPT, RECOVER = 0x0, 0x4
local ANIM_ERUPT = 6
local SOUND = asset.sound("bn6:volcano")

local ERUPT_TICKS, RECOVER_TICKS = 0x1E, 0x1E
-- The timer value the volcano erupts at.
local ERUPT_AT = 0xB

-- The panels he aims at first, by side (`off_80F1434`: an enemy navi's
-- body on the other side's area), then the other side's panels without
-- one (`byte_80F1448`); requires and forbids.
local ENEMIES = { [0] = { 0x0400_0020, 0 }, { 0x0800_0000, 0x20 } }
local EMPTY = { [0] = { 0x20, 0x0400_0000 }, { 0, 0x0800_0020 } }

-- Last, the column off the field's far edge (`dword_80F145C`, by side),
-- rows 1 to 3.
local OFF_EDGE = { [0] = 7, 0 }

local ROCKS = 5

-- What its first rock's spawn finds in r3: the row counter the column off
-- the edge was listed with, one past its last row.
local FIRST_Z = 4

-- `sub_80F13B4`: the volcano. Each rock's spawn takes the register its
-- predecessor's left: its thrower's side and flip halfword when it came,
-- else the element.
local function erupt(me: Object)
    local side = me.alliance
    local targets = panels.filtered_except(me, ENEMIES[side][1], ENEMIES[side][2])
    local empty = panels.filtered_except(me, EMPTY[side][1], EMPTY[side][2])
    if #empty > 0 then
        panels.shuffle(empty, #empty, #empty)
    end
    for _, p in empty do
        table.insert(targets, p)
    end
    for y = 1, 3 do
        local p: number = panels.pack(OFF_EDGE[side], y)
        table.insert(targets, p)
    end
    local element, damage = me.attack_element, link.damage_word(me)
    local z = FIRST_Z
    for i = 1, math.min(#targets, ROCKS) do
        local p = targets[i]
        local rock = volcano_rock.spawn(me, panels.x(p), panels.y(p), element, damage, z)
        z = if rock then me.alliance + me.flip * 0x100 else element
    end
end

-- `sub_80F1350`: the stack blows; 30 ticks after the entry, the volcano
-- erupting at 11 left.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = ANIM_ERUPT
        s.timer = ERUPT_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == ERUPT_AT then
        erupt(me)
        battle.play_sound(SOUND)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1390`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [ERUPT] = blow, [RECOVER] = recover }

return define.chip {
    id = "bn6:volcchrg",
    codes = { "C" },
    element = "fire",
    family = "fire",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 30, per_level = 10 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 404, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F1334` (`off_80F1348`).
    action = define.action {
        id = "bn6:volcchrg/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "VolcChrg (sub_80F1334)")
        end,
    },
}
