--!strict
-- The shock wave (attack object #0x16: EXE5's 0x080C739C, EXE4's 0x080CFBB0, the same code but for the panels it
-- marks; EXE6's WaveArm wave, `sub_80C6B40`, is the same number with other code): a wave that hits one panel at a time,
-- along its row. Each segment stands on its panel (a solid one: else it is gone at once) with its collision on, after
-- its variant's ticks starts the next segment one panel ahead, and goes when its animation ends (moved off the field,
-- out of sight). A variant may mark the panels it passes (crack, break, or turn them to another type); such a wave is
-- thrown, the others plain attacks; either hits navis with hit modifier 3 and no spark. Its variant
-- (`wave.variant { ... }`, a row of the game's table: its animation, its ticks on a panel and its mark) is its
-- sender's, passed on. Each game makes its own (`wave.make`): its sheet and sound, its collision types, whether a
-- segment keeps its sender (`owned`: EXE5's spawn gives it the sender's flip and the sender as its owner, 0x0800F1F6;
-- EXE4's only the sender's side, and its sprite turns by the side alone, 0x0800ACA6), and whether it notes the panel
-- ahead on its first tick of spreading (`notes_ahead`: EXE5's phase byte set to 1 as it does, EXE4's left 0).
--
-- (In EXE5's operation battle, never set in a netbattle, each segment also marks the panel ahead, 0x080E9FA4's effect
-- object #0x83, which isn't ported.)

local wave = {}

local SOLID = 0x10
local HIT_MOD = 3
-- Out of sight: 240 pixels to the right.
local GONE_X = 240

-- Its actions.
local SPREADING, FADING = 0, 4

type State = {
    -- Its variant (passed on to the next segment).
    variant: GuardWaveVariant,
}

function wave.variant(v: GuardWaveVariant): GuardWaveVariant
    return new.record("shock-wave-variant", v)
end

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- At its animation's end: out of sight, off its panels.
local function hide(me: Object): boolean
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return false
    end
    me.pos = Vec3.new(GONE_X * 0x10000, me.pos.y, me.pos.z)
    me.collision:set_region(nil)
    return true
end

-- EXE6's `sub_80C6CFC`, EXE5's 0x080C7544 (EXE4's 0x08009AEC on any mark): what the variant does to its panel.
local function mark(x: number, y: number, v: GuardWaveVariant)
    if v.panel == "cracked" then
        field.crack(x, y)
    elseif v.panel == "broken" then
        field.shatter(x, y)
    elseif v.panel then
        field.set_type(x, y, v.panel)
    end
end

-- A game's shock wave: its spawn, a segment of `owner`'s on (x, y), of `variant`, with the damage word, of no element
-- (EXE6's `sub_80C6CE4`, EXE5's 0x080C752C, EXE4's 0x080CFD2A). (Its position is the spawner's registers until its init
-- places it.)
function wave.make(look: GuardWaveLook): GuardWaveSpawn
    local spawn: GuardWaveSpawn

    -- EXE5's 0x080C74C2, EXE4's 0x080CFCDA: after its ticks, the next segment ahead; its look's end hides it.
    local function spread(me: Object)
        if look.notes_ahead and me.phase == 0 then
            me.phase = 1
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t == 0 then
            local damage = int.u32(me.damage + me.stamina * 0x10000)
            spawn(me, me.panel_x + me.facing, me.panel_y, (me.state :: State).variant, damage)
            me.action = FADING
        end
        hide(me)
    end

    -- EXE5's 0x080C7512: at its look's end, gone.
    local function fade(me: Object)
        if hide(me) then
            me:set_lifecycle("destroy")
        end
    end

    -- EXE5's 0x080C746C, EXE4's 0x080CFC84: the hits against it resolve and spark; gone with the battle or off solid
    -- ground; a hit takes it off its panel; its action; then present.
    local function tick(me: Object)
        local c = me.collision
        c:remove()
        me:hit_spark()
        if battle.over() or not field.check(me.panel_x, me.panel_y, SOLID, 0) then
            finish(me)
            return
        end
        if c.hit_flags ~= 0 then
            c:set_region(nil)
        end
        local a = me.action
        if a == SPREADING then
            spread(me)
        elseif a == FADING then
            fade(me)
        else
            error(string.format("the shock wave's action %#x reads past its table", a))
        end
        c:present()
        me:highlight_collision_panels()
    end

    -- EXE5's 0x080C73C0, EXE4's 0x080CFBD4: on a solid panel, its look and timing; its panel marked; its collision;
    -- the sound.
    local function init(me: Object)
        if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
            me:free()
            return
        end
        local v = (me.state :: State).variant
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- (`sprite_noShadow`: the first part is drawn with the sprite.)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = v.anim, v.anim
        sprite:set_animation(v.anim)
        me.timer, me.timer2 = v.ticks, 0
        sprite.palette = 0
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        mark(me.panel_x, me.panel_y, v)
        me:set_coordinates_from_panel()
        if not me:create_collision() then
            me:free()
            return
        end
        local is = if v.panel then look.thrown else look.attack
        me:setup_collision(is, look.hits, HIT_MOD)
        me.collision:set_hit_effect(nil)
        me.collision:present()
        battle.play_sound(look.sound)
        me.lifecycle = "update"
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { variant = "record:shock-wave-variant" },
        -- EXE5's 0x080C739C, EXE4's 0x080CFBB0.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }

    spawn = function(owner: Object, x: number, y: number, variant: GuardWaveVariant, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, 0, 0))
        if o then
            (o.state :: State).variant = variant
            o.panel_x, o.panel_y = x, y
            o.element = 0
            o:set_damage_word(damage)
            o.alliance = owner.alliance
            if look.owned then
                o.flip = owner.flip
                o.related1 = owner
            end
        end
        return o
    end
    return spawn
end

return wave
