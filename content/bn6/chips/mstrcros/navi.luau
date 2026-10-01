--!strict
-- The MstrCros Program Advance: MegaMan's five Crosses of his user's game,
-- each appearing by an enemy navi and using his Cross's move, then three
-- of them together. One kind (the original's actor object #0x23,
-- `sub_80BE798`): the leader the navi chip controller brings (no sprite,
-- `form` 0), who brings the Crosses (`form` their form) in two waves and
-- waits for each to finish. See docs/engine/chips.md §3.6.34.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local drill = require("../../objects/drill/drill")
local effects = require("../../lib/effects")
local erase_ray = require("../../navis/megaman/forms/erasecross/ray")
local navi_chips = require("../../lib/navi-chips/navi")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local panels = require("../../lib/panels")
local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local trajectory = require("../../lib/trajectory")

local mstrcros = {}

-- A Cross's move (its Param2): the sword, EraseCross's beam, GroundCross's
-- drill, TenguCross's fan, and the finale's charge.
export type Move = "sword" | "beam" | "drill" | "fan" | "charge"

export type State = {
    -- MegaMan's form the Cross is (Param1), or 0: the leader.
    form: number,
    move: Move,
    -- The sword's look (Param3), and the Cross's place in its wave (Param4).
    look: number,
    place: number,
    -- The leader: the controller waiting for him (ExtraVars[0]), and which
    -- of his Crosses still act (his bytes +0x6C to +0x73).
    controller: Object?,
    acting: { boolean },
    -- A Cross: his leader (whose acting flag he clears, ExtraVars[0]);
    -- whether he turns around on arriving (+0x0C); what he holds
    -- (ExtraVars[1]) and what he sends (ExtraVars[2]: the beam, the drill).
    leader: Object?,
    turn: boolean,
    held: Object?,
    child: Object?,
}

-- MegaMan's forms by number, as the navi stats give them
-- (compat/forms.toml).
local HEATCROSS, ELECCROSS, SLASHCROSS, ERASECROSS, CHARGECROSS = 1, 2, 3, 4, 5
local SPOUTCROSS, TOMAHAWKCROSS, TENGUCROSS, GROUNDCROSS, DUSTCROSS = 6, 7, 8, 9, 0xA

-- A wave's Cross: his form, move and sword look, which side of his
-- target he stands on (`beside`: 1 in front of it, 0 behind it), his
-- element.
type Entry = { form: number, move: Move, look: number, beside: number, element: number }

-- The first wave (`byte_80BE91C`, `byte_80BE936`) and the finale
-- (`byte_80BE9D8`, `byte_80BE9F3`), by the user's game.
local FIRST_WAVE: { [string]: { Entry } } = {
    gregar = {
        { form = CHARGECROSS, move = "sword", look = 0, beside = 1, element = 0 },
        { form = HEATCROSS, move = "sword", look = 2, beside = 0, element = 1 },
        { form = ELECCROSS, move = "sword", look = 4, beside = 1, element = 3 },
        { form = SLASHCROSS, move = "sword", look = 1, beside = 0, element = 0 },
        { form = ERASECROSS, move = "beam", look = 0, beside = 1, element = 0 },
    },
    falzar = {
        { form = DUSTCROSS, move = "sword", look = 0, beside = 1, element = 0 },
        { form = SPOUTCROSS, move = "sword", look = 3, beside = 0, element = 2 },
        { form = TOMAHAWKCROSS, move = "sword", look = 5, beside = 1, element = 4 },
        { form = TENGUCROSS, move = "fan", look = 0, beside = 0, element = 0 },
        { form = GROUNDCROSS, move = "drill", look = 0, beside = 1, element = 0 },
    },
}
local FINALE: { [string]: { Entry } } = {
    gregar = {
        { form = CHARGECROSS, move = "charge", look = 0, beside = 1, element = 0 },
        { form = HEATCROSS, move = "charge", look = 0, beside = 1, element = 1 },
        { form = ELECCROSS, move = "charge", look = 0, beside = 1, element = 3 },
    },
    falzar = {
        { form = DUSTCROSS, move = "charge", look = 0, beside = 1, element = 0 },
        { form = SPOUTCROSS, move = "charge", look = 0, beside = 1, element = 2 },
        { form = TOMAHAWKCROSS, move = "charge", look = 0, beside = 1, element = 4 },
    },
}

-- The sword's looks (`byte_80BEDC6`: attachments 3, 3, 0x19 to 0x1C): its
-- blade; its hit's region (`byte_80BEDA8`: the plain sword's one panel,
-- else a column); its slash (`byte_80BEDC0`: effect 0x18, else 0x16 in the
-- look's colour, the look's palette less one from look 2).
local SWORDS = {
    [0] = { blade = parts.blades.sword, region = regions.single, slash = parts.effects.slash, palette = 0 },
    [1] = { blade = parts.blades.sword, region = regions.wide, slash = parts.effects.wide, palette = 0 },
    [2] = { blade = parts.blades.fire, region = regions.wide, slash = parts.effects.wide, palette = 1 },
    [3] = { blade = parts.blades.aqua, region = regions.wide, slash = parts.effects.wide, palette = 2 },
    [4] = { blade = parts.blades.elec, region = regions.wide, slash = parts.effects.wide, palette = 3 },
    [5] = { blade = parts.blades.bamboo, region = regions.wide, slash = parts.effects.wide, palette = 4 },
}

-- GroundCross's drill arm (attachment 0x20) and TenguCross's fan (0x2A:
-- WindRack's rack) and its gust (effect 0x44).
local DRILL_ARM = attachment.look { sprite = asset.sprite("drill-arm"), palette = 0, attach_point = 19 }
local FAN = attachment.look { sprite = asset.sprite("wind-rack"), palette = 0 }
local GUST = define.effect { sprite = asset.sprite("swirl"), anim = 0, palette = 0 }

-- The finale's charge: the glow while it builds (effect 0x4E) and the
-- flash when it is full (0x4F), at the Cross's attach point 0.
local CHARGING = define.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
local CHARGED = define.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }

local SOUND_APPEAR, SOUND_CIRCLE = asset.sound("appear"), asset.sound("sun-moon")
local SOUND_BEAM, SOUND_DRILL, SOUND_FAN = asset.sound("erase-man-2"), asset.sound("drilarm"), asset.sound("windrack")
local SOUND_CHARGE, SOUND_CHARGED = asset.sound("buster-charge"), asset.sound("buster-charged")
local SOUND_FINALE = asset.sound("burst")

-- Where each place in a wave starts its circle (`byte_80BEB20`,
-- `byte_80BEC38`), 256 steps a turn.
local ANGLES = { [0] = 0, 0x55, 0xAA, 0x78, 0xC8 }

-- An enemy navi's body (`off_80BF224`), and a panel a Cross stands on
-- (solid, with no body or object).
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local SOLID, OCCUPIED = 0x10, 0x0F80_0000

-- The controller calls the leader's spawner (`sub_80BF160`) through r3,
-- which his position takes.
local SPAWNER_ADDRESS = 0x080BF161

-- A Cross's actions (`off_80BE854`); a move's is SWORD + 4 · its place in
-- `MOVES`.
local FIRST, ARRIVE, FINALE_ENTRY, SWORD, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x20
local MOVES: { Move } = { "sword", "beam", "drill", "fan", "charge" }
local MOVE_TYPE: { string } = { "sword", "beam", "drill", "fan", "charge" }

-- How many Crosses a leader keeps acting flags for.
local ACTING = 8

-- How long his phase has left, one tick down (a halfword timer).
local function tick_down(me: Object): number
    local left = me.timer - 1
    me.timer = left
    return left
end

-- Count down from `ticks` entered on the phase's first tick (counting that
-- tick too); what is left.
local function count(me: Object, ticks: number): number
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    return tick_down(me)
end

-- The game's flip value for his sprite.
local function flip_of(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- A hit (`sub_80C53A6`: it runs while dimmed).
local function hit(me: Object, x: number, y: number, z: number, region: Region)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = z,
        region = region,
        hit_effect = 0xFF,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- A Cross is done: his leader's flag for him cleared.
local function done(me: Object)
    local s = me.state :: State
    local leader = s.leader
    if leader then
        (leader.state :: State).acting[s.place + 1] = false
    end
end

-- His place on the circle around his panel's centre (`sub_80E58D2`):
-- `radius` pixels out at `angle` (forward by his facing, down the field).
local function circle(me: Object, radius: number, angle: number)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local cos, sin = trajectory.sine(angle + 64), trajectory.sine(angle)
    local p = me.pos
    me.pos = Vec3.new(cx + cos * radius * 0x100 * me.facing, cy + sin * radius * 0x100, p.z)
end

-- ---- The leader -------------------------------------------------------------

-- `sub_80BF192(beside)`: a panel beside an enemy navi (`beside` 1 in front
-- of it, 0 behind it, as the leader faces) that is his user's or a free
-- solid one, at random (one draw when there is one); none: column 0.
local function beside_enemy(me: Object, beside: number): (number, number)
    local d = 1 - 2 * bit32.bxor(me.alliance, bit32.bxor(me.flip, beside))
    local user = me.related1 :: Object
    local found: { number } = {}
    for _, p in panels.filtered(ENEMY_NAVI[me.alliance], 0) do
        local x, y = panels.x(p) + d, panels.y(p)
        if (x == user.panel_x and y == user.panel_y) or field.check(x, y, SOLID, OCCUPIED) then
            found[#found + 1] = panels.pack(x, y)
        end
    end
    if #found == 0 then
        -- (The row is what the registers held; with no column nothing
        -- reads it.)
        return 0, 0
    end
    local p = found[battle.rng_positive() % #found + 1]
    return panels.x(p), panels.y(p)
end

-- `sub_80BF160` for one of `wave`'s Crosses at place `i` (0-based), on the
-- leader's panel with his damage word, his acting flag set.
local function bring(me: Object, entry: Entry, i: number): Object?
    local s = me.state :: State
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local spec = { panel_x = me.panel_x, panel_y = me.panel_y, element = entry.element, params = { 0, 0, 0, 0 }, damage = damage }
    local o = navi_chips.spawn(mstrcros.kind, me, spec, 0)
    if not o then
        return nil
    end
    o.run_while_dimmed = true
    local cs = o.state :: State
    cs.form, cs.move, cs.look, cs.place = entry.form, entry.move, entry.look, i
    cs.leader = me
    s.acting[i + 1] = true
    return o
end

-- The user's game's table.
local function game(me: Object): string
    return if battle.navi(me.alliance).version == 0 then "gregar" else "falzar"
end

-- `sub_80BE8AE`: the first wave, all in one tick: each Cross beside his
-- target (none found: column 0) and turned to face it.
local function first_wave(me: Object)
    for i, entry in FIRST_WAVE[game(me)] do
        local o = bring(me, entry, i - 1)
        local x, y = beside_enemy(me, entry.beside)
        if o then
            o.future_panel_x, o.future_panel_y = x, y;
            (o.state :: State).turn = entry.beside == 0
        end
    end
    me.timer2 = #FIRST_WAVE[game(me)]
    me.phase, me.phase_init = 8, 0
end

-- Whether every Cross of his has finished.
local function all_done(me: Object): boolean
    local acting = (me.state :: State).acting
    for i = 1, ACTING do
        if acting[i] then
            return false
        end
    end
    return true
end

-- `sub_80BE950`: once all are done, 10 ticks.
local function wait_first(me: Object)
    if me.phase_init == 0 and not all_done(me) then
        return
    end
    if count(me, 0xA) <= 0 then
        me.timer2 = 0
        me.phase, me.phase_init = 0xC, 0
    end
end

-- `sub_80BE97A`: the finale, all in one tick, on his panel.
local function finale(me: Object)
    for i, entry in FINALE[game(me)] do
        local o = bring(me, entry, i - 1)
        if o then
            o.future_panel_x, o.future_panel_y = me.panel_x, me.panel_y
        end
    end
    me.timer2 = #FINALE[game(me)]
    me.phase, me.phase_init = 0x10, 0
end

-- `sub_80BEA10`: once all are done, 30 ticks; the controller moves on and
-- he goes.
local function wait_finale(me: Object)
    if me.phase_init == 0 and not all_done(me) then
        return
    end
    if count(me, 0x1E) > 0 then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80BE878`.
local function lead(me: Object)
    local phase = me.phase
    if phase == 0 then
        -- sub_80BE8A0
        me.timer, me.timer2 = 0, 0
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        first_wave(me)
    elseif phase == 8 then
        wait_first(me)
    elseif phase == 0xC then
        finale(me)
    elseif phase == 0x10 then
        wait_finale(me)
    else
        error(string.format("MstrCros's leader phase %#x reads past his table (off_80BE88C)", phase))
    end
end

-- ---- A Cross ------------------------------------------------------------

-- `sub_80BEA78`: 30 ticks fading in from white (places 0 to 2 appear; 3
-- and 4 stay hidden).
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = 0x1E
        if s.place < 3 then
            battle.play_sound(SOUND_APPEAR)
            me.visible = true
        end
        me.timer2 = 0x1F
        me.phase_init = 4
    end
    if tick_down(me) <= 0 then
        me.sprite.color_shader = 0
        me.phase, me.phase_init = 4, 0
        return
    end
    me.timer2 = math.max(me.timer2 - 1, 0)
    me.sprite.color_shader = me.timer2 * 0x421
end

-- `sub_80BEACE`: 21 ticks circling out from his panel's centre, a pixel
-- further and a step further round each tick.
local function circle_out(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        battle.play_sound(SOUND_CIRCLE)
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x14 then
        me.phase, me.phase_init = 8, 0
    end
    circle(me, t, ANGLES[(me.state :: State).place] + t)
end

-- `sub_80BEB28`: 30 ticks.
local function hover(me: Object)
    if count(me, 0x1E) <= 0 then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- `sub_80BEB46`: he blinks out; 4 ticks; without a target he is done
-- (his parts off), else he waits his turn.
local function blink_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if count(me, 4) > 0 then
        return
    end
    me.visible = false
    me.phase, me.phase_init = 0x10, 0
    if me.future_panel_x ~= 0 then
        return
    end
    done(me)
    me:navi_image_parts(false)
    me:set_lifecycle("destroy")
end

-- `sub_80BEB90`: 20 ticks for each place before his.
local function wait_turn(me: Object)
    if count(me, (me.state :: State).place * 0x14) <= 0 then
        me:set_action(ARRIVE)
    end
end

-- `sub_80BEA50`: the first wave's entry.
local FIRST_PHASES = { [0] = fade_in, [4] = circle_out, [8] = hover, [0xC] = blink_out, [0x10] = wait_turn }

-- `sub_80BEC84`: beside his target, turned to face it when he stands
-- behind it; 3 ticks.
local function arrive(me: Object)
    if me.phase_init == 0 then
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        if (me.state :: State).turn then
            me.flip = bit32.bxor(me.flip, 1)
        end
        me.sprite:set_flip(flip_of(me))
        me.anim = 3
        me.visible = true
    end
    if count(me, 3) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BECC6`: 5 ticks, then his move.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if count(me, 5) > 0 then
        return
    end
    local move = (me.state :: State).move
    for i, m in MOVES do
        if m == move then
            me:set_action(SWORD + 4 * (i - 1))
            return
        end
    end
end

-- `sub_80BEC68`.
local ARRIVE_PHASES = { [0] = arrive, [4] = ready }

-- `sub_80BEBD8`: the finale's Crosses appear at the end of the first
-- wave's circle (places 0 to 2 seen); 3 ticks.
local function finale_appear(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 3
        if s.place < 3 then
            battle.play_sound(SOUND_APPEAR)
            me.visible = true
        end
        circle(me, 0x14, ANGLES[s.place] + 0x14)
    end
    if count(me, 3) <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BEC40`: at once to the charge.
local function finale_ready(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0 then
        me:set_action(0x1C)
    end
end

-- `sub_80BEBBC`.
local FINALE_PHASES = { [0] = finale_appear, [4] = finale_ready }

-- `sub_80BED0C`: the sword (his look's blade); at 9 the hit on the panel
-- ahead, the slash 16 pixels up, a flash; past 21, he leaves. (The
-- action's second phase, `sub_80BEDCC`, is never entered.)
local function sword(me: Object)
    local s = me.state :: State
    local look = SWORDS[s.look]
    if not look then
        error(string.format("MstrCros's sword look %d reads past byte_80BEDC6", s.look))
    end
    if me.phase_init == 0 then
        me.anim = 5
        battle.play_sound(parts.sounds.swing)
        attachment.attach(me, look.blade, "held", { anim = 0, while_dimmed = true })
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == 9 then
        local x, y = me.panel_x + me.facing, me.panel_y
        hit(me, x, y, 0, look.region)
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), look.slash, flip_of(me), look.palette)
        battle.palette_flash(0, 0xA, true, false)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x15 then
        me:set_action(LEAVE)
    end
end

-- `sub_80BEE04`: EraseCross's beam from the panel ahead (the third kind,
-- which shuts once told: at 40 left); 70 ticks.
local function beam(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 0xF
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local ray = erase_ray.spawn(me, me.panel_x + me.facing, me.panel_y, me.element, 0, erase_ray.rays.held, damage)
        if ray then
            ray.run_while_dimmed = true
        end
        s.child = ray
        battle.play_sound(SOUND_BEAM)
        me.timer = 0x46
        battle.palette_flash(0, 0xA, true, false)
        me.phase_init = 4
    end
    -- (Without a beam the game reads a byte of the BIOS, never its third
    -- kind.)
    if me.timer == 0x28 and s.child then
        erase_ray.shut(s.child :: Object)
    end
    if tick_down(me) <= 0 then
        me:set_action(LEAVE)
    end
end

-- `sub_80BEEA2`: GroundCross's drill: his arm, the drill sent from 16
-- pixels up (it runs while dimmed), a flash, and he leans in a pixel; 30
-- ticks.
local function drill_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 0xA
        attachment.attach(me, DRILL_ARM, "held", { anim = 1, while_dimmed = true })
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        local d = drill.spawn(me, me.element, 0x10_0000, damage, { ticks = 0x1E, light = true }, "child")
        if d then
            d.run_while_dimmed = true
        end
        me.timer = 0x1E
        battle.play_sound(SOUND_DRILL)
        battle.palette_flash(0, 0xA, true, false)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y + 0x10000, p.z + 0x10000)
        me.phase_init = 4
    end
    if tick_down(me) <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BEE80`: 10 ticks, then the drill at once.
local function drill_ready(me: Object)
    if count(me, 0xA) <= 0 then
        me.phase, me.phase_init = 4, 0
        drill_out(me)
    end
end

-- `sub_80BEF02`: his arm's animation from its start (`sub_80B8E70`); 10
-- ticks.
local function drill_back(me: Object)
    if me.phase_init == 0 then
        local arm = (me.state :: State).held
        if arm then
            arm.anim = 0
            arm.anim_loaded = 0xFF
        end
    end
    if count(me, 0xA) <= 0 then
        me:set_action(LEAVE)
    end
end

-- `sub_80BEE62`.
local DRILL_PHASES = { [0] = drill_ready, [4] = drill_out, [8] = drill_back }

-- `sub_80BEF48`: TenguCross's fan and its gust; at 12 left the hit on the
-- panel ahead and a flash; to -1.
local function fan(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        battle.play_sound(SOUND_FAN)
        attachment.attach(me, FAN, "held", { anim = 0, while_dimmed = true })
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, 0x10_0000), GUST, flip_of(me))
        me.timer = 0x15
        me.phase_init = 4
    end
    if me.timer == 0xC then
        hit(me, me.panel_x + me.facing, me.panel_y, 0x10_0000, regions.wide)
        battle.palette_flash(0, 0xA, true, false)
    end
    if tick_down(me) < 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BEFC0`: 6 ticks.
local function fan_after(me: Object)
    if count(me, 5) < 0 then
        me:set_action(LEAVE)
    end
end

-- `sub_80BEF2C`.
local FAN_PHASES = { [0] = fan, [4] = fan_after }

-- An effect at his attach point 0 (`sub_8018810` by his identity).
local function at_hand(me: Object, effect: Effect): Object?
    local ax, az = me:attach_point(0)
    local p = me.pos
    return battle.effect(Vec3.new(p.x + ax * 0x10000, p.y, p.z + az * 0x10000), effect)
end

-- `sub_80BF000`: the charge: its glow for 70 ticks, then the flash; past
-- 90, on.
local function charge(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        local glow = at_hand(me, CHARGING)
        -- (A failed spawn: the game writes the timer into the BIOS.)
        if glow then
            glow.timer = 0x46
        end
        battle.play_sound(SOUND_CHARGE)
        me.phase_init = 4
    end
    if me.timer == 0x46 then
        local flash = at_hand(me, CHARGED)
        if flash then
            flash.timer = 0x14
        end
        battle.play_sound(SOUND_CHARGED)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x5A then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80BF07A`: the strike: at 4, the first Cross alone hits the whole
-- of the other side's area, bursts its panels (one every 2 ticks), shakes
-- the camera and flashes; past 60, on.
local function strike(me: Object)
    if me.phase_init == 0 then
        me.anim = 0xC
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == 4 and (me.state :: State).place == 0 then
        local area = regions.area[1 - me.alliance]
        hit(me, me.panel_x, me.panel_y, me.pos.z, area)
        -- (The game then sets flag 0x10 through what `sub_80E2FE8` returns,
        -- which is the bursts' panel count, not the object: a store into
        -- the BIOS. The bursts wait out the dimming.)
        panel_bursts.spawn(me, area, 2, 0, effects.explosion)
        battle.shake_camera(3, 0x1E)
        battle.palette_flash(0, 0x23, true, false)
        battle.play_sound(SOUND_FINALE)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 0x3C then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80BF0F8`: 31 ticks.
local function settle(me: Object)
    if count(me, 0x1E) < 0 then
        me:set_action(LEAVE)
    end
end

-- `sub_80BEFE2`.
local CHARGE_PHASES = { [0] = charge, [4] = strike, [8] = settle }

-- `sub_80BF11A`: what he holds goes; 3 ticks: out of sight, done, his
-- parts off, gone.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        local held = (me.state :: State).held
        if held then
            held:set_lifecycle("destroy")
        end
    end
    if count(me, 3) > 0 then
        return
    end
    me.visible = false
    done(me)
    me:navi_image_parts(false)
    me:set_lifecycle("destroy")
end

-- Run an action's phase from its table.
local function run_phase(me: Object, phases: { [number]: (Object) -> () }, what: string)
    local phase = phases[me.phase]
    if not phase then
        error(string.format("MstrCros's Cross's %s phase %#x reads past its table", what, me.phase))
    end
    phase(me)
end

-- `off_80BE854`, a Cross's actions.
local function act(me: Object)
    local action = me.action
    if action == FIRST then
        run_phase(me, FIRST_PHASES, "first wave")
    elseif action == ARRIVE then
        run_phase(me, ARRIVE_PHASES, "arrival")
    elseif action == FINALE_ENTRY then
        run_phase(me, FINALE_PHASES, "finale")
    elseif action == SWORD then
        if me.phase ~= 0 then
            error(string.format("MstrCros's sword phase %#x is never entered (sub_80BEDCC)", me.phase))
        end
        sword(me)
    elseif action == 0x10 then
        if me.phase ~= 0 then
            error(string.format("MstrCros's beam phase %#x reads past its table", me.phase))
        end
        beam(me)
    elseif action == 0x14 then
        run_phase(me, DRILL_PHASES, "drill")
    elseif action == 0x18 then
        run_phase(me, FAN_PHASES, "fan")
    elseif action == 0x1C then
        run_phase(me, CHARGE_PHASES, "charge")
    elseif action == LEAVE then
        leave(me)
    else
        error(string.format("MstrCros's Cross's action %#x reads past his table (off_80BE854)", action))
    end
end

-- `sub_80BE834`.
local function update(me: Object)
    if (me.state :: State).form == 0 then
        lead(me)
    else
        act(me)
    end
end

-- `sub_80BE7BC`: a Cross takes his form's image (animation 0, loaded on
-- the tick's sprite update) on his panel, on the ground, and his parts;
-- the finale's start at their entry (action 8). Then the first update at
-- once.
local function init(me: Object)
    local s = me.state :: State
    if s.form ~= 0 then
        me:wear_megaman_image(s.form)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me.sprite:set_flip(flip_of(me))
        me:navi_image_parts(true)
    end
    me:set_lifecycle("update")
    if s.form ~= 0 and s.move == "charge" then
        me.action = FINALE_ENTRY
    end
    update(me)
end

mstrcros.kind = define.kind {
    id = "mstrcros/navi",
    pool = "actor",
    state = {
        form = "u8",
        move = MOVE_TYPE,
        look = "u8",
        place = "u8",
        controller = "object",
        acting = "bool[8]",
        leader = "object",
        turn = "bool",
        held = "object",
        child = "object",
    },
    -- `sub_80BE798`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- The MstrCros chip's `navi` hook (`off_802CD5C[22]`): the leader for the
-- user, reporting to `controller`.
function mstrcros.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(mstrcros.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    o.run_while_dimmed = true
    local s = o.state :: State
    s.form = 0
    s.controller = controller
    return o
end

return mstrcros
