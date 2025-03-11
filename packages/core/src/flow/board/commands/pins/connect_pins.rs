use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use std::collections::HashSet;

use crate::{
    flow::{
        board::{Board, Command},
        pin::PinType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConnectPinsCommand {
    pub from_pin: String,
    pub to_pin: String,
    pub from_node: String,
    pub to_node: String,
}

impl ConnectPinsCommand {
    pub fn new(from_node: String, to_node: String, from_pin: String, to_pin: String) -> Self {
        ConnectPinsCommand {
            from_pin,
            to_pin,
            from_node,
            to_node,
        }
    }
}

#[async_trait]
impl Command for ConnectPinsCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        connect_pins(
            board,
            &self.from_node,
            &self.from_pin,
            &self.to_node,
            &self.to_pin,
        )?;

        let from_node = board
            .nodes
            .get(&self.from_node)
            .ok_or(anyhow::anyhow!("Node not found"))?
            .clone();

        let to_node = board
            .nodes
            .get(&self.to_node)
            .ok_or(anyhow::anyhow!("Node not found"))?
            .clone();

        board.nodes.insert(from_node.id.clone(), from_node);
        board.nodes.insert(to_node.id.clone(), to_node);

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        disconnect_pins(
            board,
            &self.from_node,
            &self.from_pin,
            &self.to_node,
            &self.to_pin,
        )?;

        let from_node = board
            .nodes
            .get(&self.from_node)
            .ok_or(anyhow::anyhow!("Node not found"))?
            .clone();

        let to_node = board
            .nodes
            .get(&self.to_node)
            .ok_or(anyhow::anyhow!("Node not found"))?
            .clone();

        board.nodes.insert(from_node.id.clone(), from_node);
        board.nodes.insert(to_node.id.clone(), to_node);

        Ok(())
    }
}

pub fn connect_pins(
    board: &mut Board,
    from_node: &str,
    from_pin: &str,
    to_node: &str,
    to_pin: &str,
) -> anyhow::Result<()> {
    if from_node == to_node {
        return Err(anyhow::anyhow!(
            "Cannot connect a node to itself".to_string()
        ));
    }

    if from_pin == to_pin {
        return Err(anyhow::anyhow!("Cannot connect a pin to itself".to_string()));
    }

    let from_node = match board.nodes.get(from_node) {
        Some(node) => node,
        None => return Err(anyhow::anyhow!("Node not found".to_string())),
    };
    let mut from_node = from_node.clone();

    let to_node = match board.nodes.get(to_node) {
        Some(node) => node,
        None => return Err(anyhow::anyhow!("Node not found".to_string())),
    };
    let mut to_node = to_node.clone();

    let from_pin = match from_node.pins.get_mut(from_pin) {
        Some(pin) => pin,
        None => {
            println!("Node: {:?}, Pin ID: {}", from_node, from_pin);
            return Err(anyhow::anyhow!("Pin not found in node".to_string()));
        }
    };

    let to_pin = match to_node.pins.get_mut(to_pin) {
        Some(pin) => pin,
        None => {
            println!("Node: {:?}, Pin ID: {}", to_node, to_pin);
            return Err(anyhow::anyhow!("Pin not found in node".to_string()));
        }
    };

    if from_pin.pin_type == PinType::Input {
        return Err(anyhow::anyhow!("Cannot connect an input pin".to_string()));
    }

    if to_pin.pin_type == PinType::Output {
        return Err(anyhow::anyhow!("Cannot connect an output pin".to_string()));
    }

    // If we would allow this, it could introduce race conditions for variable access.
    // We will allow it BUT ONLY via explicit parallel sequence node.
    if from_pin.data_type == VariableType::Execution {
        let mut old_connect_to = from_pin.connected_to.clone();
        from_pin.connected_to = HashSet::from([to_pin.id.clone()]);
        old_connect_to.remove(&to_pin.id);

        board.nodes.iter_mut().for_each(|(_, node)| {
            node.pins.iter_mut().for_each(|(_, pin)| {
                pin.depends_on.remove(&from_pin.id);
            });
        });

        to_pin.depends_on.insert(from_pin.id.clone());
    }

    if from_pin.data_type != VariableType::Execution {
        let mut old_depends_on = to_pin.depends_on.clone();
        to_pin.depends_on = HashSet::from([from_pin.id.clone()]);
        old_depends_on.remove(&from_pin.id);

        board.nodes.iter_mut().for_each(|(_, node)| {
            node.pins.iter_mut().for_each(|(_, pin)| {
                pin.connected_to.remove(&to_pin.id);
            });
        });
    }

    from_pin.connected_to.insert(to_pin.id.clone());

    board.nodes.insert(from_node.id.clone(), from_node);
    board.nodes.insert(to_node.id.clone(), to_node);
    board.fix_pins();

    Ok(())
}

pub fn disconnect_pins(
    board: &mut Board,
    from_node: &str,
    from_pin: &str,
    to_node: &str,
    to_pin: &str,
) -> anyhow::Result<()> {
    let mut from_node = match board.nodes.get(from_node) {
        Some(node) => node.clone(),
        None => return Err(anyhow::anyhow!("From Node ({}) not found", from_node)),
    };

    let mut to_node = match board.nodes.get(to_node) {
        Some(node) => node.clone(),
        None => return Err(anyhow::anyhow!("To Node ({}) not found", to_node)),
    };

    let from_pin = match from_node.pins.get_mut(from_pin) {
        Some(pin) => pin,
        None => return Err(anyhow::anyhow!("From Pin ({}) not found in node", from_pin)),
    };

    let to_pin = match to_node.pins.get_mut(to_pin) {
        Some(pin) => pin,
        None => return Err(anyhow::anyhow!("To Pin ({}) not found in node", to_pin)),
    };

    to_pin.depends_on.remove(&from_pin.id);
    from_pin.connected_to.remove(&to_pin.id);

    board.nodes.insert(from_node.id.clone(), from_node);
    board.nodes.insert(to_node.id.clone(), to_node);
    Ok(())
}
