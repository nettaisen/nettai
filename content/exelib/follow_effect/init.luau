--!strict
-- A follow effect, as EXE6, EXE5 and EXE4 share it (effect object #0x31: EXE6's `sub_80E3D68`, EXE5's 0x080E3E3C,
-- EXE4's 0x080E58B4): a sprite shown for its owner (related1), holding an effect slot, that runs its user's routine
-- each tick. The original's first parameter picks a row of the game's looks (a sprite, an animation, a palette) and
-- the routine; here each user makes its own kind (`follow_effect.make`: its look and its routine, `update`), which
-- compat maps to the original's one object. Its flip and a palette to add are its spawner's. EXE6's copies its owner's
-- visibility after the routine each tick; EXE5's and EXE4's don't (`copies_visibility`, the user's game's). While its
-- HP's bit 0 is set (a routine holding it while dimmed), its sprite stands still.
--
-- Its users: DeltaRay's bursts (EXE6's, EXE5's and EXE4's look 4, @exelib/deltaray/navi), ElecMan's glow in his
-- DElecSwd (EXE6's look 7, navis/elecman/chip) and MetalMan's fist (EXE4's look 5, chips/metalman/navi). The
-- original's other looks are spawned only by the story's navis (EXE6's looks 5 and 6, EXE5's and EXE4's looks 0 to 3
-- and their story navis' look 4), by an object no netbattle has (EXE6's look 8, attack object #0x67), or by nothing
-- (EXE6's looks 0 to 3): they aren't here.

local follow_effect = {}

export type Spec = {
    -- The kind's id (its user's).
    id: string,
    -- Its look: its row of the game's looks.
    sprite: SpriteAsset,
    anim: number,
    palette: number,
    -- Shown as its owner is after the routine (EXE6's).
    copies_visibility: boolean,
    -- Its routine, each tick.
    update: (me: Object) -> (),
}

export type FollowEffect = {
    kind: Kind,
    -- `sub_80E3FB4`: one for `owner` at `pos`, with the game's flip value `flip` and `palette_add`.
    spawn: (owner: Object, pos: Vec3, flip: number, palette_add: number?) -> Object?,
}

export type State = {
    -- The game's flip value and the palette added (its spawner's).
    flip: number,
    palette_add: number,
}

-- A user's follow effect.
function follow_effect.make(spec: Spec): FollowEffect
    -- `sub_80E3DE0`: its routine, then (EXE6's) shown as the owner is.
    local function tick(me: Object)
        spec.update(me)
        if spec.copies_visibility then
            me:copy_visibility(me.related1 :: Object)
        end
    end

    -- `sub_80E3D90`: its look, then its first update.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(spec.sprite)
        sprite.shadow = "with_sprite"
        me.anim = spec.anim
        me.anim_loaded = spec.anim
        sprite:set_animation(spec.anim)
        sprite.palette = spec.palette + s.palette_add
        sprite:set_flip(s.flip)
        me.visible = true
        me.hp = 0
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { flip = "u8", palette_add = "u8" },
        -- `sub_80E3D68`: its sprite steps unless held (HP bit 0).
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
            if bit32.band(me.hp, 1) == 0 then
                me:update_sprite()
            end
        end,
    }

    local function spawn(owner: Object, pos: Vec3, flip: number, palette_add: number?): Object?
        local o = battle.spawn(kind, pos)
        if o then
            o.related1 = owner
            local s = o.state :: State
            s.flip, s.palette_add = flip, palette_add or 0
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

-- `sub_80E3FC4`: end it (its next update frees it).
function follow_effect.stop(o: Object)
    o:set_lifecycle("destroy")
end

return follow_effect
