--!strict
-- The DS's Double Attacks (Mega Man Battle Network 5: Double Team DS's party battle: docs/design/exe5ds-map.md §5):
-- what the twelve share. A Double Attack is one of the DS's chips (0x160 to 0x16B: its record, its name for the
-- telop) with its attack, the chip's `double_attack` (`double_attack.make`): its damage by the side's level, and its
-- two navis, the actors (an actor kind of its own, the DS's T1 0x35 on: `roles`), whose pair's switch fires it
-- (rules/double_attacks). Its director (an effect kind of its own, the DS's effect object 0x96 + n; each Double
-- Attack's is the same code) is a dimming controller: the dim, the chip's telop, the course, the undim. The course: the
-- user warps out (31 ticks), both actors come, it waits for both to leave, 31 ticks, the user warps back in (31
-- ticks).
--
-- Each actor (the DS's role table, by its Param1: 0 the first, 1 the second) stands beside the user's spot and draws
-- its navi; MegaMan draws as his stats say (his form, Hub Style, his light/dark value), the active navi's stats when
-- he is on the field, else his member block's. The director links the two (each the other's partner, its +0x60), which
-- most wait on. The steps most of them share: it appears (20 ticks), charges (60 ticks, the glow at its hand), attacks,
-- waits and leaves (4 ticks). Each Double Attack's own actions are its module's.
--
-- Left out: the DS's party block +0x18 (each actor clears its navi's bit as it appears and sets it as it leaves,
-- 0x02004E7C and 0x02004E68), which only the bottom screen's party panel reads (ARM 0x020361F8: how it draws the
-- member's face): presentation, the app's party HUD's own.

local attachment = require("../objects/attachment")
local buster = require("./buster")
local collision = require("../rules/collision")
local exe5 = require("../rules/api")
local hub_shade = require("../navis/megaman/forms/hub_shade")
local megaman = require("../navis/megaman").megaman
local base_form = require("../navis/megaman/forms/base").base
local panels = require("@exelib/panels")

local double_attack = {}

-- An actor of a Double Attack: the navi it draws (its sprite and its identity: the DS's role table's sprite, its NameID
-- 0x180 + the navi's number, set as it appears), where it stands from the user's spot in pixels (x toward its front),
-- and its actions by number, the role's own table.
export type Role = {
    navi: NaviDef,
    dx: number,
    dy: number,
    actions: { [number]: (me: Object) -> () },
}

export type Spec = {
    -- Its kinds' keys' start: `<id>/director`, `<id>/actor`.
    id: string,
    -- The damage by the side's level, 0 to 9 (the DS's rows at 0x0203E4FE, ten halfwords).
    damage: { number },
    -- The two actors, the DS's role 0 and 1.
    roles: { Role },
    -- The actors' own state beside the shared fields (`State`).
    state: StateSchema?,
}

-- What every actor keeps (types.d.luau's `DoubleAttackActor`).
type State = DoubleAttackActor

-- The director's: each actor out (+0x60, +0x64: the words its actors clear as they go).
type DirectorState = { out: { boolean } }

-- The director's actions (its table: object_dimScreen, object_drawChipName, its course, object_undimScreen), and the
-- course's phases.
local DIM, NAME, COURSE, UNDIM = 0x0, 0x4, 0x8, 0xC
local WARP_OUT, ACTORS, AFTER, WARP_IN = 0x0, 0x4, 0x8, 0xC
-- Each wait of the course (one tick more than it says: counted until it falls below 0).
local COURSE_WAIT = 0x1E

-- The glow at the hand as an actor charges (the DS's new effect rows 0x6D and 0x6E: 14-08 in its animations 0 and 2,
-- for 40 and 20 ticks), with the buster's charge sounds.
local CHARGING = new.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
local CHARGED = new.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }
local CHARGING_LASTS, CHARGED_AT = 0x28, 0x14
local SOUND_CHARGING = asset.sound("buster-charge")
local SOUND_CHARGED = asset.sound("buster-charged")
local SOUND_APPEAR = asset.sound("appear")
-- How long an actor appears, and leaves.
local APPEAR_TICKS, LEAVE_TICKS = 0x14, 4
local ANIM_IDLE, ANIM_APPEAR, ANIM_LEAVE = 0, 3, 4
-- MegaMan's buster arm (the attachment table's row 0x23, EXE5's: lib/arm), which he raises in a soul's animation,
-- Hub Style's 0x0D, or the DS's own 0x0E.
local ARM = attachment.look { sprite = asset.sprite("buster-arm"), palette = 0 }

-- A count down of the object's timer: true once it reached 0 (`subs; strh; bgt`).
function double_attack.ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    return t <= 0
end

-- The same, true once it fell below 0 (`bge`): one tick more.
local function fell_below(me: Object): boolean
    local t = me.timer - 1
    me.timer = t
    return t < 0
end

-- `sub_80B8E58`: an attachment ends (its lifecycle word goes to destroy; the slot keeps the handle).
function double_attack.detach(a: Object?)
    if a then
        a:set_lifecycle("destroy")
    end
end

-- The member of `side`'s party whose block is MegaMan's (0x0200DEE8 with navi 0: none offline for side 1).
function double_attack.megaman_member(side: number): number?
    if not battle.link() and side ~= 0 then
        return nil
    end
    for m = 0, battle.party(side).count - 1 do
        local stats = battle.member(side, m)
        if stats and stats.navi.id == "megaman" then
            return m
        end
    end
    return nil
end

-- MegaMan's stats as an actor reads them (0x020A109C and its kin): the side's own while he is on the field (with his
-- Chaos Unison charge armed: AIData +0x12 of the side's navi), else his member block's.
local function megaman_stats(me: Object): (Navi, boolean)
    local side = me.alliance
    local stats = battle.navi(side)
    if stats.navi.id == "megaman" then
        -- (sub_80103BC: the side's navi; none, his block's.)
        local navi = battle.player(side)
        if navi then
            return stats, navi.b_charge_time ~= nil
        end
    end
    local m = double_attack.megaman_member(side)
    local block = if m then battle.member(side, m) else nil
    if not block then
        error("a Double Attack's MegaMan on a side whose party has no block of his (0x0200DEE8 finds none)")
    end
    return block, false
end

-- 0x020A11D0: MegaMan's sprite: on the field, his form's (`sub_800FC9E` by his form and NaviStats +0x4C); else his base
-- form's (00-00), or 00-0D in the DS's Hub Style 3, its own sprite (the GBA has none at 00-0D: his base form's stands
-- in; EXE5's Hub Styles are 1 and 2, so no setup reaches it).
local function megaman_sprite(me: Object): SpriteAsset
    local side = me.alliance
    local stats = battle.navi(side)
    if stats.navi.id == "megaman" then
        return stats.form.sprite
    end
    return base_form.sprite
end

-- 0x020A109C: what MegaMan wears, in his second related slot: in GyroSoul its propeller (the body overlays' row 2), in
-- NumberSoul its layer (`sub_80C4038`, row 1), in his base form Hub Style's shade (row 5) but in the DS's Hub Style 3.
-- Each steps even while paused (Param3 1). (A switch ends a soul as it leaves: his block is in his base form.)
local function megaman_overlay(me: Object)
    local stats = megaman_stats(me)
    local form = stats.form
    if not form.base then
        if form.id == "gyrosoul" then
            me:add_parts(form.identity :: Identity, 1)
        elseif form.id == "numbersoul" then
            me:put_on_form_overlay(form)
        end
        return
    end
    local hub = stats.hub_style
    if hub ~= 0 and hub ~= 3 then
        me:add_parts(hub_shade, 1)
    end
end

-- 0x020A1218: MegaMan's palette: in a soul 2 with his Chaos Unison charge armed, else 0; in his base form Hub Style's
-- (0x19 for style 1, 0x1E for 2) plus his light/dark value's tier (0x0800DE5C).
local function megaman_palette(me: Object): number
    local stats, armed = megaman_stats(me)
    if not stats.form.base then
        return if armed then 2 else 0
    end
    local p = 0
    local hub = stats.hub_style
    if hub == 1 then
        p = 0x19
    elseif hub == 2 then
        p = 0x1E
    end
    return p + exe5.light_dark_tier(exe5.light_dark_value(me.alliance))
end

-- 0x020A1124: MegaMan raises his buster arm (row 0x23), held in `which`, animating while dimmed: in a soul its
-- animation (its number), in palette 14 with the Chaos Unison charge armed; in Hub Style 1 animation 0x0D, in 2 that in
-- palette 14, in the DS's 3 animation 0x0E in palette 0x21 (the DS's buster arm, its own 0C-03, has fifteen
-- animations; the GBA's has fourteen: 0x0D stands in; EXE5's Hub Styles are 1 and 2, so no setup reaches it).
function double_attack.megaman_arm(me: Object, which: Slot)
    local stats, armed = megaman_stats(me)
    local anim, palette = 0, 0
    if not stats.form.base then
        anim = (stats.form.buster_arm :: { anim: number }).anim
        if armed then
            palette = 0xE
        end
    end
    local hub = stats.hub_style
    if hub == 1 then
        anim = 0xD
    elseif hub == 2 then
        anim, palette = 0xD, 0xE
    elseif hub == 3 then
        anim, palette = 0xD, 0x21
    end
    attachment.attach(me, ARM, which, { anim = anim, while_dimmed = true, palette_add = palette })
end

-- `sub_8018810` by the actor's NameID: `effect` at its attach point 0 for `ticks` ticks.
function double_attack.at_hand(me: Object, effect: Effect, ticks: number)
    local ax, az = me:attach_point(0)
    local p = me.pos
    local o = battle.effect(Vec3.new(p.x + ax * 0x10000, p.y, p.z + az * 0x10000), effect)
    if o then
        o.timer = ticks
    end
end

-- `sub_80C53A6`: a hit for this tick (`battle.hitbox`, resolving while dimmed).
function double_attack.hit(me: Object, spec: HitboxSpec): Object?
    local h = battle.hitbox(me, spec)
    if h then
        h.run_while_dimmed = true
    end
    return h
end

-- An actor appears (each role's first action): its appearing animation, 20 ticks, its NameID (its navi's identity);
-- MegaMan puts on what he wears and takes his palette; with `wears`, a team navi puts on its identity's parts
-- (`sub_8011420` with Param3 1: they step even while paused). Then it charges (`charge`) on a solid panel, else leaves
-- (`leave`); without `leave`, it charges whatever its panel.
function double_attack.appear(me: Object, charge: number, leave: number?, wears: boolean?)
    if me.phase_init == 0 then
        me.anim = ANIM_APPEAR
        me.timer = APPEAR_TICKS
        battle.play_sound(SOUND_APPEAR)
        if me.identity == megaman.identity then
            megaman_overlay(me)
            me.sprite.palette = megaman_palette(me)
        elseif wears then
            -- (sub_8011420 puts on a player's identity's parts alone: its actor record's type 2.)
            local identity = me.identity :: Identity
            if identity.actor_type == "player" then
                me:add_parts(identity, 1)
            end
        end
        me.phase_init = 4
    end
    if not double_attack.ran_out(me) then
        return
    end
    if leave == nil or field.solid(me.panel_x, me.panel_y) then
        me:set_action(charge)
    else
        me:set_action(leave)
    end
end

-- The actor's partner, the other actor its director brought (+0x60).
function double_attack.partner(me: Object): Object
    local p = (me.state :: State).partner
    if p == nil then
        error("a Double Attack's actor has no partner (the DS reads its +0x60 through a null pointer)")
    end
    return p
end

-- An actor charges for `ticks` ticks: the glow at its hand with its sound (40 ticks), the charged glow and its sound
-- `charged_at` ticks before the end (20 if none said), lasting as long. True on its last tick (the role's own then
-- follows).
function double_attack.charge(me: Object, ticks: number, charged_at: number?): boolean
    local at = charged_at or CHARGED_AT
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        double_attack.at_hand(me, CHARGING, CHARGING_LASTS)
        battle.play_sound(SOUND_CHARGING)
        me.timer = ticks
        me.phase_init = 4
    end
    if me.timer == at then
        double_attack.at_hand(me, CHARGED, at)
        battle.play_sound(SOUND_CHARGED)
    end
    return double_attack.ran_out(me)
end

-- An actor leaves: its leaving animation, 4 ticks, then gone; with `takes_off`, what it wore taken off (`sub_800F29C`
-- by its NameID, `sub_8011044`: MegaMan's).
function double_attack.leave(me: Object, takes_off: boolean)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAVE
        me.timer = LEAVE_TICKS
        me.phase_init = 4
    end
    if not double_attack.ran_out(me) then
        return
    end
    if takes_off then
        me:remove_parts(me.identity :: Identity)
    end
    me:set_lifecycle("destroy")
end

-- The panels of the other side's area but the actor's own (`object_getPanelsExceptCurrentFiltered` by its side: flag
-- 0x20 for side 0, none of it for side 1), in its side's order, packed (@exelib/panels).
function double_attack.enemy_panels(me: Object): { number }
    local side = me.alliance
    local require, forbid = if side == 0 then 0x20 else 0, if side == 0 then 0 else 0x20
    return panels.filtered_except(me, require, forbid, side)
end

-- A Double Attack's director and actors, and what its chip keeps of it (`DoubleAttack`, types.d.luau).
function double_attack.make(spec: Spec): DoubleAttack
    local roles = spec.roles
    if #roles ~= 2 then
        error(string.format("%s: a Double Attack has two actors, not %d", spec.id, #roles))
    end
    if #spec.damage ~= 10 then
        error(string.format("%s: a Double Attack's damage row has ten entries, not %d", spec.id, #spec.damage))
    end

    local state: StateSchema = { director = "object", role = "u8", partner = "object", held = "object", held2 = "object" }
    for k, v in spec.state or {} do
        state[k] = v
    end

    -- The role's action table, by its action.
    local function run(me: Object)
        local s = me.state :: State
        local role = roles[s.role + 1]
        local f = role.actions[me.action]
        if not f then
            error(string.format("%s's actor (role %d): action %#x reads past its table", spec.id, s.role, me.action))
        end
        f(me)
    end

    -- The DS's actor init (0x020A0BD8 and its copies): on the user's spot (the panel's center), stepped by the role's
    -- offset, on the ground; its navi's sprite (MegaMan's by his stats), a ground shadow, animation 0, shown, facing
    -- its side's way; then its first update at once.
    local function init(me: Object)
        local s = me.state :: State
        local role = roles[s.role + 1]
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x + role.dx * me.facing * 0x10000, p.y + role.dy * 0x10000, 0)
        local sprite = me.sprite
        if role.navi == megaman then
            sprite:load(megaman_sprite(me))
        else
            sprite:load(role.navi.sprite)
        end
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        -- (Its NameID the DS sets as it appears, this same tick.)
        me.identity = role.navi.identity
        me:set_lifecycle("update")
        run(me)
        me:update_sprite()
    end

    local actor = new.kind {
        id = spec.id .. "/actor",
        pool = "actor",
        state = state,
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                run(me)
                me:update_sprite()
            else
                -- (Its director's word for it cleared.)
                local s = me.state :: State
                local director = s.director
                if director then
                    (director.state :: DirectorState).out[s.role + 1] = false
                end
                me:free()
            end
        end,
    }

    -- The DS's actor spawner (0x020A0FEC and its copies): an actor for `role`, on the director's panel, of its user's
    -- side and facing, with its damage word, reporting to the director.
    local function bring(me: Object, role: number): Object?
        local user = me.related1 :: Object
        local o = battle.spawn(actor)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = me.panel_x, me.panel_y
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        o.related1 = user
        local s = o.state :: State
        s.role = role
        s.director = me;
        (me.state :: DirectorState).out[role + 1] = true
        return o
    end

    -- Its course (0x020A4D30's table and its copies), by phase.
    local function course(me: Object)
        local user = me.related1 :: Object
        local p = me.phase
        if p == WARP_OUT then
            if me.phase_init == 0 then
                me.timer = COURSE_WAIT
                me.phase_init = 4
                navi_chip.warp(user, true)
            end
            if fell_below(me) then
                me.phase, me.phase_init = ACTORS, 0
            end
        elseif p == ACTORS then
            local s = me.state :: DirectorState
            if me.phase_init == 0 then
                -- Each the other's partner (+0x60; D-HERO's director alone doesn't link them, and its actors never
                -- read the link).
                local first = bring(me, 0)
                local second = bring(me, 1)
                if first and second then
                    (first.state :: State).partner = second;
                    (second.state :: State).partner = first
                end
                me.phase_init = 4
            end
            if not s.out[1] and not s.out[2] then
                me.phase, me.phase_init = AFTER, 0
            end
        elseif p == AFTER then
            if me.phase_init == 0 then
                me.timer = COURSE_WAIT
                me.phase_init = 4
            end
            if fell_below(me) then
                me.phase, me.phase_init = WARP_IN, 0
            end
        elseif p == WARP_IN then
            if me.phase_init == 0 then
                navi_chip.warp(user, false)
                me.timer = COURSE_WAIT
                me.phase_init = 4
            end
            if fell_below(me) then
                me:set_action(UNDIM)
            end
        else
            error(string.format("%s's director: phase %#x reads past its table", spec.id, p))
        end
    end

    local director = new.kind {
        id = spec.id .. "/director",
        pool = "effect",
        state = { out = "bool[2]" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                dimming.begin(me)
            elseif l == "update" then
                local a = me.action
                if a == DIM then
                    dimming.dim_screen(me)
                elseif a == NAME then
                    dimming.show_telop(me)
                elseif a == COURSE then
                    course(me)
                elseif a == UNDIM then
                    dimming.undim_screen(me)
                else
                    error(string.format("%s's director: action %#x reads past its table", spec.id, a))
                end
            else
                dimming.finish(me)
            end
        end,
    }

    -- The DS's spawner (0x020A4DF8 and its copies): the director on `user`'s panel, of its side and facing, with the
    -- chip's element and the damage word. (Its position is what the launcher's registers held: its course never reads
    -- it.)
    local function spawn(user: Object, element: number, damage: number): Object?
        local o = battle.spawn(director)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = user.panel_x, user.panel_y
        o.element = element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        return o
    end

    return new.record("exe5-double-attack", {
        navis = { roles[1].navi, roles[2].navi },
        damage = spec.damage,
        spawn = spawn,
    })
end

return double_attack
