--!strict
-- EraseMan's own chip, EDeletBm (the original's `sub_80F1056`, his action
-- table's entry 0xA): he raises his scythe (10 ticks), slashes an erase
-- beam (chips/eraseman/beam) onto each panel ahead in his row, to the
-- field's edge and five at most, which stand while he stays in the action
-- (60 ticks), then recovers for 30. See docs/engine/standard-chips.md,
-- "Action 0x0A".

local beam = require("../../chips/eraseman/beam")
local link = require("../../lib/link_chips")

export type State = { timer: number }

local STATE = { timer = "u16" }

local RAISE, SLASH, RECOVER = 0x0, 0x4, 0x8
local ANIM_RAISE, ANIM_SLASH = 0x11, 0x12
local SOUND = asset.sound("erase-man-2")

-- The beams: aimed straight, 60 ticks, his own (they go when his action
-- does, and stand still while dimmed); five at most.
local BEAM: EraseBeam = { aim = 1, ticks = 0x3C, navis = true }
local MAX_BEAMS = 5

-- `sub_80F1074`: the scythe raised; 9 ticks after the entry.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = ANIM_RAISE
        s.timer = 9
    elseif link.run_out(s) then
        link.go(me, SLASH)
    end
end

-- `sub_80F1148`: a beam on each panel ahead while on the field. The
-- spawner's registers (his row, the element and 1.0) are each beam's
-- position until its init, which keeps its Z's fraction.
local function beams(me: Object)
    for n = 1, MAX_BEAMS do
        local x, y = me.panel_x + me.facing * n, me.panel_y
        if not field.valid(x, y) then
            return
        end
        local element = me.attack_element
        beam.spawn(me, x, y, element, BEAM, link.damage_word(me), Vec3.new(y, element, 0x1_0000))
    end
end

-- `sub_80F10A0`: the slash; 60 ticks.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_SLASH
        beams(me)
        battle.play_sound(SOUND)
        s.timer = 0x3C
    elseif link.run_out(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F10CE`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = 0x1E
    elseif link.run_out(s) then
        me.anim = 0
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [RAISE] = raise, [SLASH] = slash, [RECOVER] = recover }

return define.chip {
    id = "edeletbm",
    name = "EDeletBm",
    description = "Pierce\nfwd and\nfire beam",
    codes = { "K" },
    element = "null",
    family = "cursor",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 30, per_level = 20 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 403, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F1056` (`off_80F1068`).
    action = define.action {
        id = "edeletbm/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "EDeletBm (sub_80F1056)")
        end,
    },
}
