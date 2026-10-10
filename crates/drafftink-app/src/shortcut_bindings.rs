//! One editable registry; custom bindings are routed through the existing commands.
use crate::settings::UserSettings;
use drafftink_core::{input::InputState, tools::ToolKind};
use winit::{event::KeyEvent, keyboard::{Key, KeyCode, NamedKey, PhysicalKey}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context { Canvas, Text, Math, Controls }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope { Canvas, Text, Math, Shared }
#[derive(Clone, Debug)]
pub struct Definition {
    pub id: &'static str,
    pub default: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub scope: Scope,
    pub tool: Option<ToolKind>,
}
impl Definition {
    pub fn binding<'a>(&self, settings: &'a UserSettings) -> &'a str {
        if let Some(tool) = self.tool { settings.shortcut_for(tool) }
        else { settings.shortcut_overrides.get(self.id).map(String::as_str).unwrap_or(self.default) }
    }
    pub fn set(&self, settings: &mut UserSettings, value: String) {
        if let Some(tool) = self.tool { *settings.shortcut_for_mut(tool) = value; }
        else { settings.shortcut_overrides.insert(self.id.into(), value); }
    }
    fn active(&self, context: Context) -> bool {
        match self.scope { Scope::Canvas=>context==Context::Canvas, Scope::Text=>context==Context::Text, Scope::Math=>context==Context::Math, Scope::Shared=>context!=Context::Controls }
    }
}
pub fn definitions() -> Vec<Definition> {
    let mut list = Vec::new();
    for (id, key, label, tool) in [
        ("select","S","Sélection",ToolKind::Select), ("pan","H","Pan",ToolKind::Pan),
        ("rectangle","R","Rectangle",ToolKind::Rectangle), ("ellipse","O","Formes — répéter pour changer",ToolKind::Ellipse),
        ("arrow","A","Flèche",ToolKind::Arrow), ("line","L","Ligne",ToolKind::Line),
        ("draw","D","Draw",ToolKind::Freehand), ("highlighter","K","Highlighter",ToolKind::Highlighter),
        ("eraser","E","Eraser — Classic / Manual",ToolKind::Eraser), ("text","T","Text",ToolKind::Text),
        ("math","M","Math",ToolKind::Math), ("laser","Z","Laser — normal / permanent",ToolKind::LaserPointer),
    ] { list.push(Definition { id, default:key, description:label, category:"Outils", scope:Scope::Canvas, tool:Some(tool) }); }
    for (id, key, label, scope) in [
        ("pin","Ctrl+L","Épingler / détacher la sélection",Scope::Canvas),
        ("reset_panels","Ctrl+Shift+R","Réinitialiser la disposition des panneaux",Scope::Canvas),
        ("properties","F1","Afficher / masquer Properties",Scope::Canvas),
        ("presentation","Ctrl+P","Mode présentation",Scope::Shared),
        ("fullscreen","F11","Plein écran",Scope::Shared),
        ("save_png","Ctrl+S","Enregistrer le PNG",Scope::Shared),
        ("save_json","Ctrl+Shift+S","Exporter le JSON",Scope::Shared),
        ("open","Ctrl+O","Ouvrir un document",Scope::Canvas),
        ("export_png","Ctrl+E","Exporter en PNG",Scope::Canvas),
        ("copy_png","Ctrl+Shift+E","Copier la sélection en PNG",Scope::Canvas),
        ("copy_png_alias","Ctrl+Shift+C","Copier la sélection en PNG (alternative)",Scope::Canvas),
        ("undo","Ctrl+Z","Undo",Scope::Shared), ("redo","Ctrl+Shift+Z","Redo",Scope::Shared), ("redo_alias","Ctrl+Y","Redo (alternative)",Scope::Shared),
        ("select_all","Ctrl+A","Tout sélectionner",Scope::Shared), ("copy","Ctrl+C","Copier",Scope::Shared), ("cut","Ctrl+X","Couper",Scope::Shared), ("paste","Ctrl+V","Coller",Scope::Shared),
        ("duplicate","Ctrl+D","Dupliquer la sélection",Scope::Canvas),
        ("group","Ctrl+G","Grouper",Scope::Canvas), ("ungroup","Ctrl+Shift+G","Dégrouper",Scope::Canvas),
        ("delete","Delete","Supprimer la sélection",Scope::Canvas), ("delete_alias","Backspace","Supprimer (alternative)",Scope::Canvas),
        ("cancel","Escape","Annuler l’action / revenir à la sélection",Scope::Canvas),
        ("zoom_in","=","Zoom +",Scope::Canvas), ("zoom_in_alias","Plus","Zoom + (alternative)",Scope::Canvas), ("zoom_out","-","Zoom −",Scope::Canvas), ("zoom_reset","0","Zoom initial",Scope::Canvas), ("zoom_fit","F","Cadrer la sélection ou le tableau",Scope::Canvas),
        ("help","?","Keyboard Shortcuts",Scope::Canvas),
        ("nudge_left","ArrowLeft","Déplacer légèrement à gauche",Scope::Canvas), ("nudge_right","ArrowRight","Déplacer légèrement à droite",Scope::Canvas), ("nudge_up","ArrowUp","Déplacer légèrement en haut",Scope::Canvas), ("nudge_down","ArrowDown","Déplacer légèrement en bas",Scope::Canvas),
        ("fast_left","Shift+ArrowLeft","Déplacer rapidement à gauche",Scope::Canvas), ("fast_right","Shift+ArrowRight","Déplacer rapidement à droite",Scope::Canvas),
        ("fast_up","Ctrl+ArrowUp","Déplacer rapidement en haut",Scope::Canvas), ("fast_down","Ctrl+ArrowDown","Déplacer rapidement en bas",Scope::Canvas),
        ("fast_up_alias","Shift+ArrowUp","Déplacement rapide en haut (alternative)",Scope::Canvas), ("fast_down_alias","Shift+ArrowDown","Déplacement rapide en bas (alternative)",Scope::Canvas),
        ("bold","Ctrl+B","Gras",Scope::Text), ("italic","Ctrl+I","Italique",Scope::Text), ("underline","Ctrl+U","Souligner",Scope::Text),
        ("overline","Ctrl+Shift+B","Barre au-dessus de la sélection",Scope::Text), ("vector","Ctrl+Shift+V","Vecteur au-dessus de la sélection",Scope::Text),
        ("superscript","Shift+ArrowUp","Exposant dans Text",Scope::Text), ("subscript","Shift+ArrowDown","Indice dans Text",Scope::Text),
        ("math_superscript","Ctrl+ArrowUp","Exposant dans Math / formule",Scope::Math), ("math_subscript","Ctrl+ArrowDown","Indice dans Math / formule",Scope::Math),
    ] { list.push(Definition { id, default:key, description:label, category:if scope==Scope::Text||scope==Scope::Math {"Texte et formules"} else {"Tableau et fichiers"}, scope, tool:None }); }
    list
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding { pub key: String, pub ctrl: bool, pub shift: bool, pub alt: bool }
fn key_name(key: &str) -> String {
    match key.trim().to_lowercase().as_str() {
        "up"|"arrowup"|"↑"=>"ArrowUp".into(), "down"|"arrowdown"|"↓"=>"ArrowDown".into(),
        "left"|"arrowleft"|"←"=>"ArrowLeft".into(), "right"|"arrowright"|"→"=>"ArrowRight".into(),
        "esc"|"escape"=>"Escape".into(), "del"|"delete"=>"Delete".into(), "backspace"=>"Backspace".into(),
        "+"|"plus"=>"Plus".into(), "space"=>"Space".into(),
        other if other.chars().count()==1=>other.to_uppercase(),
        other=>other.to_uppercase(),
    }
}
fn implicit_shift(key: &str) -> bool { matches!(key,"Plus"|"?"|"!"|"@"|"#"|"$"|"%"|"&"|"*"|"("|")"|"_"|":"|"\""|"<"|">"|"{"|"}") }
impl Binding {
    pub fn parse(value: &str) -> Option<Self> {
        let mut result=Self{key:String::new(),ctrl:false,shift:false,alt:false};
        if value.trim()=="+" { result.key="Plus".into(); return Some(result); }
        for token in value.split('+').map(str::trim) {
            match token.to_lowercase().as_str() {
                "ctrl"|"control"|"ctl"=>result.ctrl=true, "shift"=>result.shift=true, "alt"=>result.alt=true,
                ""=>return None,
                _ if result.key.is_empty()=>result.key=key_name(token), _=>return None,
            }
        }
        if result.key.is_empty() {return None;}
        if implicit_shift(&result.key) { result.shift=false; }
        let allowed=result.key.chars().count()==1 || matches!(result.key.as_str(),"ArrowUp"|"ArrowDown"|"ArrowLeft"|"ArrowRight"|"Escape"|"Delete"|"Backspace"|"Space"|"Plus") || result.key.strip_prefix('F').and_then(|v|v.parse::<u8>().ok()).is_some_and(|n|(1..=12).contains(&n));
        allowed.then_some(result)
    }
    pub fn format(&self) -> String {
        let mut parts=Vec::new(); if self.ctrl {parts.push("Ctrl");} if self.shift {parts.push("Shift");} if self.alt {parts.push("Alt");} parts.push(&self.key);parts.join("+")
    }
}
pub fn conflicts(settings: &UserSettings) -> Vec<String> {
    let list=definitions(); let mut errors=Vec::new();
    for (index,def) in list.iter().enumerate() {
        let value=def.binding(settings); if value.trim().is_empty(){continue;}
        let Some(binding)=Binding::parse(value) else {errors.push(format!("{} : raccourci invalide",def.description));continue;};
        for previous in &list[..index] {
            let overlap=previous.scope==def.scope || previous.scope==Scope::Shared || def.scope==Scope::Shared;
            if overlap && Binding::parse(previous.binding(settings)).is_some_and(|b|b==binding) {errors.push(format!("{} : déjà utilisé par {}",binding.format(),previous.description));}
        }
    }
    errors
}
pub enum Remap { None, Mapped, Blocked }
pub fn remap_event(event: &mut KeyEvent, input: &mut InputState, settings: &UserSettings, context: Context) -> Remap {
    if context==Context::Controls || event.state!=winit::event::ElementState::Pressed { return Remap::None; }
    let key=match &event.logical_key { Key::Character(c)=>key_name(c), Key::Named(n)=>key_name(&format!("{n:?}")), _=>return Remap::None };
    let actual=Binding{shift:input.shift()&&!implicit_shift(&key),key,ctrl:input.ctrl(),alt:input.alt()};
    let definitions=definitions();
    if let Some(def)=definitions.iter().find(|def|def.active(context)&&Binding::parse(def.binding(settings)).is_some_and(|binding|binding==actual)) {
        let original=Binding::parse(def.default).unwrap();
        input.override_shortcut_modifiers(original.ctrl,original.shift,original.alt);
        let key=if let Some(tool)=def.tool { format!("@tool:{tool:?}") } else { original.key.clone() };
        event.logical_key=logical_key(&key);
        event.physical_key=PhysicalKey::Code(physical_key(&original.key));
        event.text=None;
        Remap::Mapped
    } else if definitions.iter().any(|def|def.active(context)&&Binding::parse(def.default).is_some_and(|binding|binding==actual)&&Binding::parse(def.binding(settings)).as_ref()!=Some(&actual)) { Remap::Blocked }
    else { Remap::None }
}
fn logical_key(key: &str) -> Key {
    let named=match key {"ArrowUp"=>Some(NamedKey::ArrowUp),"ArrowDown"=>Some(NamedKey::ArrowDown),"ArrowLeft"=>Some(NamedKey::ArrowLeft),"ArrowRight"=>Some(NamedKey::ArrowRight),"Escape"=>Some(NamedKey::Escape),"Delete"=>Some(NamedKey::Delete),"Backspace"=>Some(NamedKey::Backspace),"Space"=>Some(NamedKey::Space),"F1"=>Some(NamedKey::F1),"F2"=>Some(NamedKey::F2),"F3"=>Some(NamedKey::F3),"F4"=>Some(NamedKey::F4),"F5"=>Some(NamedKey::F5),"F6"=>Some(NamedKey::F6),"F7"=>Some(NamedKey::F7),"F8"=>Some(NamedKey::F8),"F9"=>Some(NamedKey::F9),"F10"=>Some(NamedKey::F10),"F11"=>Some(NamedKey::F11),"F12"=>Some(NamedKey::F12),_=>None};
    named.map(Key::Named).unwrap_or_else(||Key::Character((if key=="Plus" {"+"} else {key}).into()))
}
fn physical_key(key: &str) -> KeyCode {
    match key {"ArrowUp"=>KeyCode::ArrowUp,"ArrowDown"=>KeyCode::ArrowDown,"ArrowLeft"=>KeyCode::ArrowLeft,"ArrowRight"=>KeyCode::ArrowRight,"="|"Plus"=>KeyCode::Equal,"-"=>KeyCode::Minus,"?"=>KeyCode::Slash,"F1"=>KeyCode::F1,"F11"=>KeyCode::F11,"A"=>KeyCode::KeyA,"B"=>KeyCode::KeyB,"C"=>KeyCode::KeyC,"D"=>KeyCode::KeyD,"E"=>KeyCode::KeyE,"F"=>KeyCode::KeyF,"G"=>KeyCode::KeyG,"H"=>KeyCode::KeyH,"I"=>KeyCode::KeyI,"K"=>KeyCode::KeyK,"L"=>KeyCode::KeyL,"M"=>KeyCode::KeyM,"O"=>KeyCode::KeyO,"P"=>KeyCode::KeyP,"R"=>KeyCode::KeyR,"S"=>KeyCode::KeyS,"T"=>KeyCode::KeyT,"U"=>KeyCode::KeyU,"V"=>KeyCode::KeyV,"X"=>KeyCode::KeyX,"Y"=>KeyCode::KeyY,"Z"=>KeyCode::KeyZ,_=>KeyCode::F24}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_registry_and_portable_bindings() {
        let mut settings=UserSettings::default();
        assert!(conflicts(&settings).is_empty());
        for id in ["pin","reset_panels","overline","vector","save_json","fast_up"] { assert!(definitions().iter().any(|d|d.id==id)); }
        let pin=definitions().into_iter().find(|d|d.id=="pin").unwrap();
        pin.set(&mut settings,"Alt+L".into());
        let restored:UserSettings=serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(pin.binding(&restored),"Alt+L");
        assert_eq!(Binding::parse("ctrl + shift + r").unwrap().format(),"Ctrl+Shift+R");
        assert_eq!(Binding::parse("shift+arrowup").unwrap().key,"ArrowUp");
        pin.set(&mut settings,"Ctrl+S".into());
        assert!(!conflicts(&settings).is_empty());
    }
}
