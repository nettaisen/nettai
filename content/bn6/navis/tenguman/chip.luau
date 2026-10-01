--!strict
-- TenguMan's own chip, FTornado (the original's `sub_80F1A46`, his action
-- table's entry 0xA): a tornado (navis/tenguman/tornado) on every panel
-- ahead of him with an enemy body or object or a neutral object, a hit
-- over the whole field (which, at panel (0, 0), never lands), and a gust
-- of wind on him; 30 ticks, then 20 of recovery. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("../../lib/regions")
local tornado = require("./tornado")

export type State = { timer: number }

local STATE = { timer = "u16" }

local BLOW, RECOVER = 0x0, 0x4
local ANIM_BLOW, ANIM_RECOVER = 0x10, 7
local SOUND = asset.sound("aqua-surge")

-- The gust on him (32 pixels up).
local GUST = define.effect { sprite = asset.sprite("swirl"), anim = 1, palette = 1 }
local GUST_Z = 0x20_0000

local BLOW_TICKS, RECOVER_TICKS = 0x1E, 0x14

-- Where tornadoes rise, by side (`byte_80F1B70`): the other side's bodies
-- and objects, and neutral objects.
local TARGETS = { [0] = 0x0580_0000, 0x0A80_0000 }

-- What a tornado's spawn finds in r3: the address of `byte_80F1B70`, which
-- its spawner just read, whose low half its Z keeps.
local TORNADO_Z = 0x080F_1B70

-- `sub_80F1B24`: a tornado on each panel ahead of him that has a target,
-- rows 1 to 3, columns 1 to 6.
local function tornadoes(me: Object)
    local element, damage = me.attack_element, link.damage_word(me)
    for y = 1, 3 do
        for x = 1, 6 do
            if me.facing * (me.panel_x - x) < 0 and bit32.btest(field.flags(x, y), TARGETS[me.alliance]) then
                tornado.spawn(me, x, y, element, damage, Vec3.new(y, element, TORNADO_Z))
            end
        end
    end
end

-- `sub_80F1B78`: a hit of no damage over the whole field, at panel (0,
-- 0): off the field, it goes before it registers.
local function field_hit(me: Object)
    battle.hitbox(me, {
        panel_x = 0,
        panel_y = 0,
        element = 0,
        z = 0,
        region = regions.field,
        hit_effect = nil,
        target = collision.navi_target,
        self_type = collision.navi,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
    })
end

-- `sub_80F1A60`: the tornadoes, the field's hit, the gust; 30 ticks.
local function blow(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_BLOW
        link.add_bonus(me)
        tornadoes(me)
        field_hit(me)
        battle.play_sound(SOUND)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, GUST_Z), GUST, link.flip(me))
        s.timer = BLOW_TICKS
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F1AB6`: 20 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RECOVER
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES = { [BLOW] = blow, [RECOVER] = recover }

return define.chip {
    id = "ftornado",
    name = "FTornado",
    description = "Attk enmy\nw/spnning\ntwister!",
    codes = { "T" },
    element = "null",
    family = "wind",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 10, per_level = 10 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 407, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F1A46` (`off_80F1A58`).
    action = define.action {
        id = "ftornado/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "FTornado (sub_80F1A46)")
        end,
    },
}
