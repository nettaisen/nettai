--!strict
-- DblHero's heroes (the original's actor object #0x1F, `sub_80BCD14`;
-- docs/engine/dimming-chip-effects.md §17): a copy of the user, MegaMan,
-- and ProtoMan, who stand where the user stood while the user vanishes.
-- ProtoMan fades in from white; both charge for 60 ticks; then MegaMan
-- fires ten volleys at every enemy panel while ProtoMan slashes along (his
-- slashes hit nothing); MegaMan gives the user back once ProtoMan is gone.
-- MegaMan shares the user's actor data, so his buster arm is the user's.

local stand_in = require("@exelib/dimming/stand_in")
local attachment = require("../../objects/attachment/attachment")
local buster = require("../../lib/buster")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local parts = require("../../lib/swords/parts")
local protoman = require("../../navis/protoman/navi")

local heroes = {}

export type State = {
    -- The chip's controller (@exelib/dimming/stand_in; none for ProtoMan).
    controller: Object?,
    -- ProtoMan (Param1 1), else MegaMan's copy.
    protoman: boolean,
    -- The other hero (ExtraVars+0x10).
    partner: Object?,
    -- Their attachments (ExtraVars+4, +0xC).
    held: Object?,
    held2: Object?,
    -- ProtoMan's whiteness as he appears (Unk_0c, 0x1F down to 0).
    white: number,
}

-- ProtoMan's look and identity (which his parts and attach points come
-- by).
local PROTOMAN_SPRITE = asset.sprite("bn6:protoman")
local PROTOMAN = protoman.identity
-- Where each stands from the user's spot (`dword_80BCE44`, pixels; x
-- toward its front).
local OFFSET: { [string]: { number } } = { megaman = { -10, -10 }, protoman = { 10, 10 } }

-- The charge (effects 0x4E and 0x4F at its attach point 0, for 40 and 20
-- ticks).
local CHARGING = define.effect { sprite = asset.sprite("bn6:charge-glow"), anim = 0, palette = 0 }
local CHARGED = define.effect { sprite = asset.sprite("bn6:charge-glow"), anim = 2, palette = 0 }
local CHARGE_TICKS, CHARGED_AT, CHARGING_LASTS = 0x3C, 0x14, 0x28
local SOUND_CHARGING = asset.sound("bn6:buster-charge")
local SOUND_CHARGED = asset.sound("bn6:buster-charged")
-- Ten volleys (or slashes), 15 ticks apart, flashing on their first,
-- sixth and eleventh ticks; then 31 ticks.
local VOLLEYS, VOLLEY_TICKS, FLASH_AT, FLASH_AT_2, AFTER = 10, 0xF, 0xA, 5, 0x1E
local SOUND_VOLLEY = asset.sound("bn6:buster-shot")
local SOUND_SLASH = asset.sound("bn6:sword-swing")
local SOUND_APPEAR = asset.sound("bn6:appear")
-- MegaMan's flash, ProtoMan's (effects 0x21, 0x16).
local FLASH = { megaman = effects.flash, protoman = parts.effects.wide }
-- ProtoMan's sword (attachment row 3, animation 0xB).
local ANIM_IDLE, ANIM_LEAVE, ANIM_SLASH, ANIM_VOLLEY = 0, 4, 5, 0xE
local PROTOMAN_LEAVES = 4
-- The volley's shots (`sub_80C53A6`, 0x0405FF00 + the region): an attack
-- hitting navis with no spark, hit modifier 3, resolving while dimmed,
-- over the rest of the row from the enemy's first panel
-- (`byte_80BD2CC`, by the panels to the far edge: one to six; none for
-- none or seven).
local NO_SPARK: Spark? = nil
local SHOT_HIT_MOD = 3
local SHOT_REGIONS: { [number]: Region? } = {
    [1] = regions.single,
    [2] = regions.long,
    [3] = regions.line,
    [4] = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 } } },
    [5] = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 } } },
    [6] = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } } },
}

local function who(s: State): string
    return if s.protoman then "protoman" else "megaman"
end

-- `sub_80B8E58`: an attachment ends (its lifecycle word goes to
-- destroy; the slot keeps the handle).
local function detach(a: Object?)
    if a then
        a:set_lifecycle("destroy")
    end
end

local function partner_of(me: Object): Object
    local p = (me.state :: State).partner
    if not p then
        error("DblHero's MegaMan has no ProtoMan (its waits read through a null pointer)")
    end
    return p
end

-- `sub_80BD278`: along row `y` from the panel in front, past its side's
-- panels, the first other panel, and the shot's region by its panels to
-- the far edge.
local function shot_from(me: Object, y: number): (number, Region?)
    local d = me.facing
    local x = me.panel_x + d
    while field.valid(x, y) do
        local p = field.panel(x, y) :: Panel
        if p.alliance ~= me.alliance then
            break
        end
        x += d
    end
    local edge = if d > 0 then 7 else 0
    return x, SHOT_REGIONS[math.abs(edge - x)]
end

-- `sub_80BD24C`: a shot down each row.
local function shoot(me: Object)
    for y = 1, 3 do
        local x, region = shot_from(me, y)
        local h = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = 0,
            region = region,
            hit_effect = NO_SPARK,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = SHOT_HIT_MOD,
        })
        if h then
            h.run_while_dimmed = true
        end
    end
end

-- `sub_80BD2D8`: a flash on a panel of the other side's from the column in
-- front on, at random (a `rng_positive` draw).
local function flash(me: Object)
    local s = me.state :: State
    local list: { { number } } = {}
    local x = me.panel_x + me.facing
    while field.valid(x, 1) do
        for y = 1, 3 do
            local p = field.panel(x, y) :: Panel
            if p.alliance ~= me.alliance then
                table.insert(list, { x, y })
            end
        end
        x += me.facing
    end
    if #list == 0 then
        error("DblHero's flash: no panel of the other side's (sub_80BD2D8 divides by zero)")
    end
    local p = list[battle.rng_positive() % #list + 1]
    local cx, cy = field.center(p[1], p[2])
    battle.effect(Vec3.new(cx, cy, 0), FLASH[who(s)], bit32.bxor(me.alliance, me.flip))
end

-- An effect at its attach point 0 (`sub_8018810`) for `ticks` ticks.
local function at_hand(me: Object, fx: Effect, ticks: number)
    local ax, az = me:attach_point(0)
    local p = me.pos
    local o = battle.effect(Vec3.new(p.x + ax * 0x1_0000, p.y, p.z + az * 0x1_0000), fx)
    if o then
        o.timer = ticks
    end
end

-- `sub_80BCEB2` (both): the charge, 60 ticks; then the attachment slots
-- let go and the next action.
local function charge(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        at_hand(me, CHARGING, CHARGING_LASTS)
        battle.play_sound(SOUND_CHARGING)
        me.timer = CHARGE_TICKS
        me.phase_init = 4
    end
    if me.timer == CHARGED_AT then
        at_hand(me, CHARGED, CHARGED_AT)
        battle.play_sound(SOUND_CHARGED)
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local s = me.state :: State
    s.held, s.held2 = nil, nil
    me.action, me.phase, me.phase_init = me.action + 4, 0, 0
end

-- `sub_80BCF58` / `sub_80BD134`: one tick, then ten.
local function count(me: Object)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.timer2 = VOLLEYS
    me.phase, me.phase_init = 4, 0
end

-- The flashes on a volley's (or slash's) sixth and eleventh ticks; at its
-- end the next, or the last phase.
local function volley_tail(me: Object)
    if me.timer == FLASH_AT then
        flash(me)
    end
    if me.timer == FLASH_AT_2 then
        flash(me)
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local n = me.timer2 - 1
    me.timer2 = n
    if n > 0 then
        me.phase_init = 0
        return
    end
    me.phase, me.phase_init = 8, 0
end

-- `sub_80BCF7A`: a volley: the user's buster arm (by the user's actor
-- data) and a muzzle flash, the form overlay restarted, the shots, a
-- flash.
local function volley(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_VOLLEY, 0xFF
        me.timer = VOLLEY_TICKS
        battle.play_sound(SOUND_VOLLEY)
        detach(s.held)
        buster.raise_arm_for(me.related1 :: Object, me, "held", true)
        detach(s.held2)
        attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
        -- sub_80C44D2: the form overlay restarts.
        local overlay = me.related2
        if overlay then
            overlay.anim_loaded = 0xFF
            overlay:step_sprite()
        end
        shoot(me)
        flash(me)
        me.phase_init = 4
    end
    volley_tail(me)
end

-- `sub_80BD156`: a slash (no hit), with ProtoMan's sword.
local function slash(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_SLASH, 0xFF
        detach(s.held)
        attachment.attach(me, parts.blades.sword, "held", { anim = 0xB, while_dimmed = true })
        battle.play_sound(SOUND_SLASH)
        me.timer = VOLLEY_TICKS
        flash(me)
        me.phase_init = 4
    end
    volley_tail(me)
end

-- `sub_80BCFF6`: once ProtoMan is going, both attachments off, back on
-- the user's spot; 31 ticks; then the user back.
local function megaman_done(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if partner_of(me).lifecycle ~= "destroy" then
            return
        end
        me.anim = ANIM_IDLE
        detach(s.held)
        detach(s.held2)
        me.pos = (me.related1 :: Object).pos
        me:set_panel_from_coordinates()
        me.timer = AFTER
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        stand_in.leave(me)
    end
end

-- `sub_80BCE90`: MegaMan waits for ProtoMan to charge (then charges too,
-- at once) or to give up (then to the end).
local function megaman_wait(me: Object)
    local a = partner_of(me).action
    if a == 4 then
        me.action = 4
        charge(me)
    elseif a == 0xC then
        me.action, me.phase, me.phase_init = 8, 8, 0
    end
end

-- `sub_80BD064`: ProtoMan appears, fading from white and blinking for 16
-- ticks; after 31 more he charges, on a solid panel, else leaves.
local function protoman_appear(me: Object)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        -- sub_80BD084.
        if me.phase_init == 0 then
            me.sprite.color_shader = 0x7FFF
            s.white = 0x1F
            battle.play_sound(SOUND_APPEAR)
            me.timer = 0
            me.phase_init = 4
        else
            me.visible = true
            local t = me.timer + 1
            me.timer = t
            if bit32.btest(t, 1) then
                me.visible = false
            end
            local w = s.white - 2
            s.white = w
            if w <= 0 then
                me.visible = true
                s.white = 0
                me.phase, me.phase_init = 4, 0
            end
        end
    elseif phase == 4 then
        -- sub_80BD0DA.
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        else
            local t = me.timer - 1
            me.timer = t
            if t <= 0 then
                local a = if field.solid(me.panel_x, me.panel_y) then 4 else 0xC
                me.action, me.phase, me.phase_init = a, 0, 0
            end
        end
    else
        error(string.format("DblHero's ProtoMan: appearing phase %#x reads past off_80BD07C", phase))
    end
    -- sub_80BD374: the white as a color shader.
    me.sprite.color_shader = s.white * 0x421
end

-- `sub_80BD1B8`: 31 ticks after the slashes, then leave.
local function protoman_after(me: Object)
    if me.phase_init == 0 then
        me.timer = AFTER
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.action, me.phase, me.phase_init = 0xC, 0, 0
    end
end

-- `sub_80BD1DA`: the sword off, 5 ticks, gone. (His parts stay on.)
local function protoman_leave(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAVE
        detach((me.state :: State).held)
        me.timer = PROTOMAN_LEAVES
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

-- `sub_80BCE4C`: by hero and action (`off_80BCE74`, `off_80BCE80`).
local function tick(me: Object)
    local s = me.state :: State
    local a = me.action
    if not s.protoman then
        if a == 0 then
            megaman_wait(me)
        elseif a == 4 then
            charge(me)
        elseif a == 8 then
            local phase = me.phase
            if phase == 0 then
                count(me)
            elseif phase == 4 then
                volley(me)
            elseif phase == 8 then
                megaman_done(me)
            else
                error(string.format("DblHero's MegaMan: phase %#x reads past off_80BCF4C", phase))
            end
        else
            error(string.format("DblHero's MegaMan: action %#x reads past off_80BCE74", a))
        end
        return
    end
    if a == 0 then
        protoman_appear(me)
    elseif a == 4 then
        charge(me)
    elseif a == 8 then
        local phase = me.phase
        if phase == 0 then
            count(me)
        elseif phase == 4 then
            slash(me)
        elseif phase == 8 then
            protoman_after(me)
        else
            error(string.format("DblHero's ProtoMan: phase %#x reads past off_80BD128", phase))
        end
    elseif a == 0xC then
        protoman_leave(me)
    else
        error(string.format("DblHero's ProtoMan: action %#x reads past off_80BCE80", a))
    end
end

-- `sub_80BD20A`: a hero for `user` on (`x`, `y`) with `element` and the
-- damage word `damage`, of the user's side and facing and with its
-- NameID, running while dimmed.
local function spawn(user: Object, x: number, y: number, element: number, damage: number, protoman: boolean): Object?
    local o = battle.spawn(heroes.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.identity = user.identity
    o.run_while_dimmed = true
    (o.state :: State).protoman = protoman
    return o
end

-- `sub_80BCD38`: drawn as the user (MegaMan's copy, in the user's palette)
-- or as ProtoMan, beside the user's spot; its identity's parts on; the
-- user vanishes. MegaMan's copy brings ProtoMan. On at once.
local function init(me: Object)
    local s = me.state :: State
    local user = me.related1 :: Object
    if s.protoman then
        me.identity = PROTOMAN
    end
    local sprite = me.sprite
    if s.protoman then
        sprite:load(PROTOMAN_SPRITE)
    else
        sprite:load_like(user)
    end
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    local off = OFFSET[who(s)]
    local p = user.pos
    me.pos = Vec3.new(p.x + off[1] * me.facing * 0x1_0000, p.y + off[2] * 0x1_0000, p.z)
    me:set_panel_from_coordinates()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    if not s.protoman then
        sprite.palette = user.sprite.palette
    end
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts_of(me, false, true)
    dimming.hide_user(user)
    if not s.protoman then
        local proto = spawn(user, me.panel_x, me.panel_y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), true)
        if proto then
            (proto.state :: State).partner = me
        end
        s.partner = proto
    end
    me:set_lifecycle("update")
    tick(me)
end

heroes.kind = define.kind {
    id = "bn6:dblhero/heroes",
    pool = "actor",
    state = {
        controller = "object",
        protoman = "bool",
        partner = "object",
        held = "object",
        held2 = "object",
        white = "u8",
    },
    -- `sub_80BCD14`: by state, then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80BD20A` with MegaMan's copy: the heroes for `user`, from
-- `controller` (busy until MegaMan is done), with the damage word
-- `damage`.
function heroes.spawn(controller: Object, user: Object, damage: number): Object?
    local o = spawn(user, controller.panel_x, controller.panel_y, controller.element, damage, false)
    if o then
        (o.state :: State).controller = controller
        ;(controller.state :: { busy: boolean }).busy = true
    end
    return o
end

return heroes
