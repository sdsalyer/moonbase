//! # Phase 4 Demo: Option Negotiation State Machine (RFC 1143)
//!
//! This example demonstrates RFC 1143 Q-method telnet option negotiation,
//! including loop prevention, state tracking, and queue system.
//! Run with: `cargo run --example phase4_demo`

use telnet_negotiation::{
    NegotiationResult, OptionNegotiator, OptionState, Side, TelnetOption, TelnetSequence,
};

fn print_state(negotiator: &OptionNegotiator, option: TelnetOption) {
    println!(
        "   States: Local={:?}, Remote={:?}",
        negotiator.get_state(Side::Local, option),
        negotiator.get_state(Side::Remote, option)
    );
    println!(
        "   Enabled: Local={}, Remote={}",
        negotiator.is_enabled(Side::Local, option),
        negotiator.is_enabled(Side::Remote, option)
    );
}

fn handle_result(description: &str, result: &NegotiationResult) {
    println!("   {}: {:?}", description, result.new_state);
    if let Some(response) = &result.response {
        match response {
            TelnetSequence::Negotiation { command, option } => {
                println!("     -> Send: {:?} {:?}", command, option);
            }
            _ => println!("     -> Send: {:?}", response),
        }
    }
    if let Some(error) = &result.error {
        println!("     ERROR: {}", error);
    }
    println!("     Enabled: {}", result.enabled);
}

fn main() {
    println!("=== Telnet Negotiation Library - Phase 4 Demo ===\n");
    println!("RFC 1143 Q-Method Option Negotiation State Machine\n");

    let mut negotiator = OptionNegotiator::new();

    println!("1. Initial state - all options disabled:");
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n2. Basic negotiation: We request remote to enable ECHO");
    let result = negotiator.request_enable(Side::Remote, TelnetOption::ECHO);
    handle_result("Request enable", &result);
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n3. Remote agrees with WILL ECHO:");
    let result = negotiator.handle_will(TelnetOption::ECHO);
    handle_result("Handle WILL", &result);
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n4. Remote requests local side to enable ECHO (DO ECHO):");
    let result = negotiator.handle_do(TelnetOption::ECHO);
    handle_result("Handle DO", &result);
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n5. We decide to disable our ECHO:");
    let result = negotiator.request_disable(Side::Local, TelnetOption::ECHO);
    handle_result("Request disable", &result);
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n6. Remote acknowledges with DONT ECHO:");
    let result = negotiator.handle_dont(TelnetOption::ECHO);
    handle_result("Handle DONT", &result);
    print_state(&negotiator, TelnetOption::ECHO);

    println!("\n{}", "=".repeat(60));
    println!("7. RFC 1143 Queue System Demo");
    println!("{}", "=".repeat(60));

    let mut negotiator2 = OptionNegotiator::new();

    println!("\n   Starting negotiation to enable NAWS:");
    let result = negotiator2.request_enable(Side::Remote, TelnetOption::NAWS);
    handle_result("Request enable NAWS", &result);

    println!("\n   While negotiating, user wants to disable (queue test):");
    let result = negotiator2.request_disable(Side::Remote, TelnetOption::NAWS);
    handle_result("Request disable while negotiating", &result);
    if let OptionState::WantYes { queue } = result.new_state {
        println!("     Queue state: {:?}", queue);
    }

    println!("\n   Remote confirms with WILL NAWS (queue activates):");
    let result = negotiator2.handle_will(TelnetOption::NAWS);
    handle_result("Handle WILL (with queue)", &result);
    print_state(&negotiator2, TelnetOption::NAWS);

    println!("\n{}", "=".repeat(60));
    println!("8. RFC 1143 Loop Prevention Demo");
    println!("{}", "=".repeat(60));

    let mut negotiator3 = OptionNegotiator::new();

    println!("\n   Simulating potential loop scenario from RFC 1143...");

    println!("\n   Step 1: We request to enable ECHO:");
    let result = negotiator3.request_enable(Side::Local, TelnetOption::ECHO);
    handle_result("Request enable", &result);

    println!("\n   Step 2: Instead of DO, remote sends DONT (violation):");
    let result = negotiator3.handle_dont(TelnetOption::ECHO);
    handle_result("Handle DONT (should be DO)", &result);

    println!("\n   RFC 1143 prevents infinite loops that would occur in RFC 854");

    println!("\n{}", "=".repeat(60));
    println!("9. Option Acceptance Policy Demo");
    println!("{}", "=".repeat(60));

    let mut negotiator4 = OptionNegotiator::new();

    // Test various option acceptance
    let test_options = [
        (TelnetOption::ECHO, "Should accept"),
        (TelnetOption::NAWS, "Should accept"),
        (TelnetOption::TERMINAL_TYPE, "Should accept"),
        (TelnetOption::LOGOUT, "Should reject"),
        (TelnetOption::MCCP2, "Should reject (complex)"),
        (TelnetOption::GMCP, "Should accept (MUD)"),
    ];

    for (option, expectation) in test_options {
        println!("\n   Remote requests: WILL {:?} ({})", option, expectation);
        let result = negotiator4.handle_will(option);
        let response = match &result.response {
            Some(TelnetSequence::Negotiation { command, .. }) => format!("{:?}", command),
            _ => "None".to_string(),
        };
        println!("     Response: {}, Accepted: {}", response, result.enabled);
    }

    println!("\n{}", "=".repeat(60));
    println!("10. Complete Bidirectional Negotiation");
    println!("{}", "=".repeat(60));

    let mut negotiator5 = OptionNegotiator::new();

    // Simulate a complete telnet session startup
    println!("\n   Typical BBS server startup sequence:");

    let negotiations = [
        (
            "Server: IAC WILL ECHO",
            Side::Local,
            TelnetOption::ECHO,
            true,
        ),
        (
            "Server: IAC WILL SUPPRESS_GO_AHEAD",
            Side::Local,
            TelnetOption::SUPPRESS_GO_AHEAD,
            true,
        ),
        (
            "Server: IAC DO NAWS",
            Side::Remote,
            TelnetOption::NAWS,
            true,
        ),
        (
            "Server: IAC DO TERMINAL_TYPE",
            Side::Remote,
            TelnetOption::TERMINAL_TYPE,
            true,
        ),
    ];

    for (description, side, option, enable) in negotiations {
        println!("\n   {}", description);
        let result = if enable {
            negotiator5.request_enable(side, option)
        } else {
            negotiator5.request_disable(side, option)
        };

        if let Some(response) = &result.response {
            if let TelnetSequence::Negotiation { command, option } = response {
                println!("     -> Sends: IAC {:?} {:?}", command, option);
            }
        }
    }

    // Show summary
    println!("\n   Negotiation summary:");
    let (local_options, remote_options) = negotiator5.get_enabled_options();
    println!("     Local enabled options: {:?}", local_options);
    println!("     Remote enabled options: {:?}", remote_options);

    println!("\n{}", "=".repeat(60));
    println!("Phase 4 Demo Complete!");
    println!("{}", "=".repeat(60));
    println!("Demonstrated:");
    println!("• RFC 1143 compliant state machine");
    println!("• Loop prevention mechanisms");
    println!("• Queue system for rapid option changes");
    println!("• Automatic response generation");
    println!("• Option acceptance policies");
    println!("• Complete bidirectional negotiation");
    println!("\nReady for Phase 5: TelnetStream Integration");
}
