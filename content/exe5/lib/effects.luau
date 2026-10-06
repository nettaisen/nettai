--!strict
-- EXE5's one-shot effects (`battle.effect(pos, effects.explosion)`): rows of EXE5's effect table (0x080DFC94: a
-- sprite, its animation and palette), which effect object 0 (SpawnT4BattleObjectWithId0) plays once. Its rows to
-- 0x22 are EXE6's (`byte_80E0398`) in EXE5's sprites, and the shared code shows them by the same numbers; past them
-- the two tables differ. Named as content/exe6/lib/effects names EXE6's rows; effects one owner shows live with it.

local effects = {}

local EXPLOSION = asset.sprite("explosion")
local SMALL_PUFF = asset.sprite("small-puff")
local PUFF = asset.sprite("puff")

-- Row 0: an explosion: a blast landing, something breaking.
effects.explosion = new.effect { sprite = EXPLOSION, anim = 0, palette = 0 }

-- Row 1: a splash of bubbles: something landing in a hole.
effects.splash = new.effect { sprite = SMALL_PUFF, anim = 0, palette = 0 }

-- Row 2: dust (a rock breaking), bubbles (Otenko's statue breaking, AntiDmg's shuriken landing), a small splash
-- (something diving into or out of the field).
effects.dust = new.effect { sprite = SMALL_PUFF, anim = 1, palette = 0 }
effects.bubbles = new.effect { sprite = SMALL_PUFF, anim = 1, palette = 0 }
effects.ripple = new.effect { sprite = SMALL_PUFF, anim = 1, palette = 0 }

-- Row 3: a navi deleted (and a soul's change and revert).
effects.deletion = new.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }

-- Row 5: a burst: a meteor or a fist landing.
effects.burst = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }

-- Row 6: HP recovered.
effects.heal = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 }

-- Row 0x12: a burst of smoke: a navi coming or going in a puff.
effects.smoke = new.effect { sprite = PUFF, anim = 0, palette = 0 }

-- Row 0x14: a puff of smoke: a thrown thing gone when the battle ends, a field object a chip removes.
effects.puff = new.effect { sprite = PUFF, anim = 2, palette = 0 }

-- Row 0x1E: the flash where a side cut in.
effects.cut_in_flash = new.effect { sprite = asset.sprite("hit-marker"), anim = 0, palette = 0 }

-- Row 0x21: a flash: a gun's shot landing.
effects.flash = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 }

-- Row 0x22: a fiery burst: a bullet's hit, a sprung trap's counterattack.
effects.shell_burst = new.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 }

-- Row 0x2B (EXE6's 0x46): the mark over a navi whose trap sprang (AntiNavi, AntiRecv).
effects.trap_mark = new.effect { sprite = asset.sprite("hit-sparks"), anim = 7, palette = 0 }

return effects
